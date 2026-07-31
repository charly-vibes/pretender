//! Primitive obsession and boolean cluster detection.
//!
//! Detects functions with excessive bare boolean parameters and
//! primitive types used in place of domain types.

use crate::model::{CodeUnit, Language, UnitKind};

/// Result of primitive-obsession analysis for one code unit.
#[derive(Debug, Clone, Default)]
pub struct PrimitiveObsessionAnalysis {
    pub bool_param_count: u32,
    pub violations: Vec<PrimitiveViolation>,
}

/// A single violation finding.
#[derive(Debug, Clone)]
pub struct PrimitiveViolation {
    pub kind: PrimitiveViolationKind,
    #[allow(dead_code)] // informative metadata for future output formats
    pub unit_name: String,
    #[allow(dead_code)] // informative metadata for future output formats
    pub param_name: String,
    #[allow(dead_code)] // informative metadata for future output formats
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveViolationKind {
    BoolCluster,
    PrimitiveDomainParam,
}

/// Analyze code units for primitive-obsession violations.
///
/// * `bool_cluster_max` — max number of bare bool params before flagging (0 = disabled)
/// * `primitive_param_check` — check for String/int params with domain-like names
/// * `extra_domain_patterns` — additional domain name patterns from config
pub fn analyze(
    units: &[CodeUnit],
    _language: &Language,
    bool_cluster_max: u32,
    primitive_param_check: bool,
    extra_domain_patterns: &[String],
) -> Vec<PrimitiveObsessionAnalysis> {
    let mut results = Vec::new();

    for unit in units {
        let mut analysis = PrimitiveObsessionAnalysis::default();

        // Skip struct/class/record definitions — only check function/method params
        if !matches!(unit.kind, UnitKind::Ctor | UnitKind::Initializer) {

        // Count bool params
        for param in &unit.parameters {
            let is_bool = param
                .type_name
                .as_deref()
                .map(|t| {
                    let t = t.trim();
                    t == "bool"
                        || t == "boolean"
                        || t == "Boolean"
                        || t == "bool?"
                        || t == "Bool"
                        || t == "boolish"
                })
                .unwrap_or(false);

            if is_bool {
                analysis.bool_param_count += 1;
            }

            // Check for primitive domain params
            if primitive_param_check {
                if let Some(ref type_name) = param.type_name {
                    let type_trimmed = type_name.trim();
                    let is_primitive = matches!(
                        type_trimmed,
                        "String" | "str" | "int" | "i32" | "i64" | "u32" | "u64" | "Int"
                            | "Integer" | "number" | "string"
                    );
                    if is_primitive && is_domain_param_name(&param.name, extra_domain_patterns) {
                        analysis.violations.push(PrimitiveViolation {
                            kind: PrimitiveViolationKind::PrimitiveDomainParam,
                            unit_name: unit.name.clone(),
                            param_name: param.name.clone(),
                            line: param.span.start_line,
                        });
                    }
                }
            }
        }

        // Check bool cluster threshold
        if bool_cluster_max > 0 && analysis.bool_param_count >= bool_cluster_max {
            for param in &unit.parameters {
                let is_bool = param
                    .type_name
                    .as_deref()
                    .map(|t| {
                        let t = t.trim();
                        t == "bool"
                            || t == "boolean"
                            || t == "Boolean"
                            || t == "bool?"
                            || t == "Bool"
                    })
                    .unwrap_or(false);
                if is_bool {
                    analysis.violations.push(PrimitiveViolation {
                        kind: PrimitiveViolationKind::BoolCluster,
                        unit_name: unit.name.clone(),
                        param_name: param.name.clone(),
                        line: param.span.start_line,
                    });
                }
            }
        }

        } // end of non-ctor block

        results.push(analysis);
    }

    results
}

/// Check if a parameter name suggests a domain concept that should be a type.
///
/// Excludes common false positives: `id`, `name`, `path` — these are
/// legitimate primitive uses in most contexts.
fn is_domain_param_name(name: &str, extra_patterns: &[String]) -> bool {
    let domain_patterns = [
        "email",
        "url",
        "uri",
        "date",
        "timestamp",
        "phone",
        "address",
        "zip",
        "ssn",
        "uuid",
        "token",
        "secret",
    ];

    let lower = name.to_ascii_lowercase();
    domain_patterns
        .iter()
        .copied()
        .chain(extra_patterns.iter().map(|s| s.as_str()))
        .any(|pattern| lower.contains(pattern))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Parameter, Span};

    fn make_param(name: &str, type_name: Option<&str>) -> Parameter {
        Parameter {
            name: name.to_string(),
            span: Span {
                start_line: 1,
                end_line: 1,
            },
            type_name: type_name.map(|s| s.to_string()),
        }
    }

    fn make_unit(name: &str, kind: UnitKind, params: Vec<Parameter>) -> CodeUnit {
        CodeUnit {
            name: name.to_string(),
            kind,
            span: Span {
                start_line: 1,
                end_line: 5,
            },
            parameters: params,
            body: crate::model::Block {
                span: Span {
                    start_line: 1,
                    end_line: 5,
                },
                nesting: 0,
                children: vec![],
            },
            is_exported: false,
            assertions: 0,
            parent_class: None,
        }
    }

    #[test]
    fn bool_cluster_exceeds_threshold() {
        let unit = make_unit(
            "do_something",
            UnitKind::Function,
            vec![
                make_param("a", Some("bool")),
                make_param("b", Some("bool")),
                make_param("c", Some("bool")),
                make_param("d", Some("bool")),
            ],
        );
        let results = analyze(&[unit], &Language::Rust, 3, false, &[]);
        assert!(!results[0].violations.is_empty());
        assert!(results[0].violations.iter().any(|v| matches!(
            v.kind,
            PrimitiveViolationKind::BoolCluster
        )));
        assert_eq!(results[0].bool_param_count, 4);
    }

    #[test]
    fn bool_cluster_within_threshold() {
        let unit = make_unit(
            "do_something",
            UnitKind::Function,
            vec![
                make_param("a", Some("bool")),
                make_param("b", Some("bool")),
            ],
        );
        let results = analyze(&[unit], &Language::Rust, 3, false, &[]);
        assert!(results[0].violations.is_empty());
    }

    #[test]
    fn struct_definition_not_flagged() {
        // Constructors should be skipped
        let unit = make_unit("new", UnitKind::Ctor, vec![
            make_param("debug", Some("bool")),
            make_param("verbose", Some("bool")),
            make_param("dry_run", Some("bool")),
        ]);
        let results = analyze(&[unit], &Language::Rust, 3, false, &[]);
        assert!(results[0].violations.is_empty());
    }

    #[test]
    fn domain_param_detected() {
        let unit = make_unit(
            "register_user",
            UnitKind::Function,
            vec![make_param("email", Some("String"))],
        );
        let results = analyze(&[unit], &Language::Rust, 0, true, &[]);
        assert!(!results[0].violations.is_empty());
        assert!(matches!(
            results[0].violations[0].kind,
            PrimitiveViolationKind::PrimitiveDomainParam
        ));
    }

    #[test]
    fn common_names_excluded() {
        let unit = make_unit(
            "get_user",
            UnitKind::Function,
            vec![
                make_param("user_id", Some("String")),
                make_param("name", Some("string")),
                make_param("path", Some("String")),
            ],
        );
        let results = analyze(&[unit], &Language::Rust, 0, true, &[]);
        assert!(results[0].violations.is_empty());
    }

    #[test]
    fn no_type_annotations_no_violation() {
        // Parameters without type annotations should not trigger
        let unit = make_unit(
            "do_something",
            UnitKind::Function,
            vec![make_param("x", None)],
        );
        let results = analyze(&[unit], &Language::Rust, 3, true, &[]);
        assert!(results[0].violations.is_empty());
        assert_eq!(results[0].bool_param_count, 0);
    }

    #[test]
    fn extra_patterns_detected() {
        let unit = make_unit(
            "process",
            UnitKind::Function,
            vec![make_param("api_key", Some("String"))],
        );
        let results = analyze(&[unit], &Language::Rust, 0, true, &["api_key".to_string()]);
        assert!(!results[0].violations.is_empty());
    }
}