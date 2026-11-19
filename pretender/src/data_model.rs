//! Data-model checks: `excessive_fields` and `primitive_field`.
//!
//! Pure analysis over the universal model's type definitions; no I/O and
//! no config types — callers pass plain threshold values. Findings carry
//! everything the main.rs wiring needs to build `ViolationReport`s
//! (metric id, subject, span, actual, limit).

use crate::model::{Span, TypeDefinition};
use crate::primitive_obsession::{is_bare_primitive_type, is_domain_param_name};

/// Metric id for the field-count check.
pub const EXCESSIVE_FIELDS: &str = "excessive_fields";
/// Metric id for the field primitive-obsession check.
pub const PRIMITIVE_FIELD: &str = "primitive_field";

/// One data-model finding. Fields are read by the main.rs wiring when it
/// builds `ViolationReport`s; the pure module only constructs them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataModelViolation {
    pub metric: &'static str,
    pub subject: String,
    pub span: Span,
    pub actual: u32,
    pub limit: u32,
}

/// Analyze type definitions for data-model violations.
///
/// * `excessive_fields_max` — max field count before flagging (0 = disabled)
/// * `primitive_field_check` — check fields for bare primitives with domain-like names
/// * `domain_patterns` — additional domain name patterns from config
pub fn analyze(
    type_defs: &[TypeDefinition],
    excessive_fields_max: u32,
    primitive_field_check: bool,
    domain_patterns: &[String],
) -> Vec<DataModelViolation> {
    let mut violations = Vec::new();

    for type_def in type_defs {
        if excessive_fields_max > 0 && type_def.fields.len() as u32 >= excessive_fields_max {
            violations.push(DataModelViolation {
                metric: EXCESSIVE_FIELDS,
                subject: type_def.name.clone(),
                span: type_def.span.clone(),
                actual: type_def.fields.len() as u32,
                limit: excessive_fields_max,
            });
        }

        if primitive_field_check {
            for field in &type_def.fields {
                let is_primitive = field
                    .type_name
                    .as_deref()
                    .is_some_and(is_bare_primitive_type);
                if is_primitive && is_domain_param_name(&field.name, domain_patterns) {
                    violations.push(DataModelViolation {
                        metric: PRIMITIVE_FIELD,
                        subject: format!("{}.{}", type_def.name, field.name),
                        span: field.span.clone(),
                        actual: 1,
                        limit: 1,
                    });
                }
            }
        }
    }

    violations
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Field, Language, TypeKind};

    fn field(name: &str, type_name: Option<&str>) -> Field {
        Field {
            name: name.to_string(),
            type_name: type_name.map(|s| s.to_string()),
            span: Span {
                start_line: 2,
                end_line: 2,
            },
        }
    }

    fn type_def(name: &str, kind: TypeKind, fields: Vec<Field>) -> TypeDefinition {
        TypeDefinition {
            name: name.to_string(),
            kind,
            span: Span {
                start_line: 1,
                end_line: 12,
            },
            fields,
            language: Language::Rust,
        }
    }

    fn fields_of_type(type_name: &str, count: usize) -> Vec<Field> {
        (0..count)
            .map(|i| field(&format!("{type_name}_{i}"), Some(type_name)))
            .collect()
    }

    fn count_fields(count: usize) -> Vec<Field> {
        (0..count)
            .map(|i| field(&format!("data{i}"), Some("u32")))
            .collect()
    }

    #[test]
    fn excessive_fields_at_threshold_flags_one() {
        let big = type_def("Config", TypeKind::Struct, count_fields(10));
        let small = type_def("Point", TypeKind::Struct, count_fields(3));
        let violations = analyze(&[big, small], 10, false, &[]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].metric, "excessive_fields");
        assert_eq!(violations[0].subject, "Config");
        assert_eq!(violations[0].actual, 10);
        assert_eq!(violations[0].limit, 10);
        assert_eq!(violations[0].span.start_line, 1);
    }

    #[test]
    fn excessive_fields_zero_disables() {
        let big = type_def("Config", TypeKind::Struct, count_fields(10));
        assert!(analyze(&[big], 0, false, &[]).is_empty());
    }

    #[test]
    fn excessive_fields_below_threshold_not_flagged() {
        let td = type_def("Config", TypeKind::Struct, count_fields(9));
        assert!(analyze(&[td], 10, false, &[]).is_empty());
    }

    #[test]
    fn primitive_field_flagged_when_enabled() {
        let td = type_def(
            "User",
            TypeKind::Struct,
            vec![field("email", Some("String"))],
        );
        let violations = analyze(&[td], 0, true, &[]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].metric, "primitive_field");
        assert_eq!(violations[0].subject, "User.email");
        assert_eq!(violations[0].actual, 1);
        assert_eq!(violations[0].limit, 1);
        assert_eq!(violations[0].span.start_line, 2);
    }

    #[test]
    fn primitive_field_disabled_yields_none() {
        let td = type_def(
            "User",
            TypeKind::Struct,
            vec![field("email", Some("String"))],
        );
        assert!(analyze(&[td], 0, false, &[]).is_empty());
    }

    #[test]
    fn primitive_field_extra_domain_pattern_respected() {
        let td = type_def(
            "Job",
            TypeKind::Class,
            vec![field("run_id", Some("String"))],
        );
        // No base pattern matches `run_id`.
        assert!(analyze(std::slice::from_ref(&td), 0, true, &[]).is_empty());
        // The extra pattern does.
        let violations = analyze(&[td], 0, true, &["run".to_string()]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].metric, "primitive_field");
        assert_eq!(violations[0].subject, "Job.run_id");
    }

    #[test]
    fn primitive_field_requires_bare_primitive() {
        let domain_typed = type_def(
            "User",
            TypeKind::Struct,
            vec![field("email", Some("Email"))],
        );
        assert!(analyze(&[domain_typed], 0, true, &[]).is_empty());

        let plain_named = type_def(
            "User",
            TypeKind::Struct,
            vec![field("note", Some("String"))],
        );
        assert!(analyze(&[plain_named], 0, true, &[]).is_empty());

        let untyped = type_def("User", TypeKind::Struct, vec![field("email", None)]);
        assert!(analyze(&[untyped], 0, true, &[]).is_empty());
    }

    #[test]
    fn primitive_field_multiple_types_and_fields() {
        let user = type_def(
            "User",
            TypeKind::Class,
            fields_of_type("String", 2), // names: String_0, String_1
        );
        let order = type_def(
            "Order",
            TypeKind::Record,
            vec![field("token", Some("i32"))],
        );
        let violations = analyze(&[user, order], 0, true, &[]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].subject, "Order.token");
    }
}
