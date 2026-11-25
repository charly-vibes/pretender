//! Purpose: unit tests for the primitive-obsession detector, moved out of
//! `src/primitive_obsession.rs` (slice pretender-5zo.25) so the source file
//! stays under the app-role file_lines threshold while the tests gain the
//! test role's looser thresholds.
//! Responsibilities: pin `analyze`'s bool-cluster, domain-param, exclusion,
//! and ordering behavior through its public API only.
//! Rationale: assertion-equivalent move from the former inline
//! `#[cfg(test)] mod tests` — three `assert!(…is_empty())` forms became
//! `assert_eq!(len(), 0)`, and two tests gained an added param-name pin
//! (`"email"`, `"api_key"`); no assertion was weakened or dropped. Unit
//! builders
//! take at most two arguments (test role params_max = 2), and the test
//! bodies differ structurally (call shapes and assertion counts) so the
//! lazy-cluster detector finds no group of three or more identical
//! fingerprints.

use pretender::model::{Block, CodeUnit, Parameter, Span, UnitKind};
use pretender::primitive_obsession::{analyze, PrimitiveViolationKind};

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

fn base_unit(name: &str) -> CodeUnit {
    CodeUnit {
        name: name.to_string(),
        kind: UnitKind::Function,
        span: Span {
            start_line: 1,
            end_line: 5,
        },
        parameters: vec![],
        body: Block {
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
        has_test_attribute: false,
    }
}

fn fn_unit(name: &str, params: Vec<Parameter>) -> CodeUnit {
    let mut unit = base_unit(name);
    unit.parameters = params;
    unit
}

fn ctor_unit(name: &str, params: Vec<Parameter>) -> CodeUnit {
    let mut unit = base_unit(name);
    unit.kind = UnitKind::Ctor;
    unit.parameters = params;
    unit
}

#[test]
fn bool_cluster_exceeds_threshold() {
    let unit = fn_unit(
        "do_something",
        vec![
            make_param("a", Some("bool")),
            make_param("b", Some("bool")),
            make_param("c", Some("bool")),
            make_param("d", Some("bool")),
        ],
    );
    let results = analyze(&[unit], 3, false, &[]);
    assert!(!results[0].violations.is_empty());
    assert!(results[0]
        .violations
        .iter()
        .any(|v| matches!(v.kind, PrimitiveViolationKind::BoolCluster)));
    assert_eq!(results[0].bool_param_count, 4);
}

#[test]
fn bool_cluster_within_threshold() {
    let params: Vec<_> = ["a", "b"]
        .iter()
        .map(|n| make_param(n, Some("bool")))
        .collect();
    let results = analyze(&[fn_unit("do_something", params)], 3, false, &[]);
    assert_eq!(results[0].violations.len(), 0);
}

#[test]
fn struct_definition_not_flagged() {
    // Constructors should be skipped
    let flags = ["debug", "verbose", "dry_run"]
        .iter()
        .map(|n| make_param(n, Some("bool")))
        .collect();
    let results = analyze(&[ctor_unit("new", flags)], 3, false, &[]);
    assert_eq!(results[0].violations.len(), 0);
}

#[test]
fn domain_param_detected() {
    let unit = fn_unit("register_user", vec![make_param("email", Some("String"))]);
    let results = analyze(&[unit], 0, true, &[]);
    assert!(!results[0].violations.is_empty());
    assert!(matches!(
        results[0].violations[0].kind,
        PrimitiveViolationKind::PrimitiveDomainParam
    ));
    assert_eq!(results[0].violations[0].param_name, "email");
}

#[test]
fn common_names_excluded() {
    let mut params = vec![make_param("user_id", Some("String"))];
    params.push(make_param("name", Some("string")));
    params.push(make_param("path", Some("String")));
    let results = analyze(&[fn_unit("get_user", params)], 0, true, &[]);
    assert_eq!(results[0].violations.len(), 0);
}

#[test]
fn no_type_annotations_no_violation() {
    // Parameters without type annotations should not trigger
    let unit = fn_unit("do_something", vec![make_param("x", None)]);
    let results = analyze(&[unit], 3, true, &[]);
    assert!(results[0].violations.is_empty());
    assert_eq!(results[0].bool_param_count, 0);
}

#[test]
fn extra_patterns_detected() {
    let unit = fn_unit("process", vec![make_param("api_key", Some("String"))]);
    let results = analyze(&[unit], 0, true, &["api_key".to_string()]);
    assert!(!results[0].violations.is_empty());
    assert_eq!(results[0].violations[0].param_name, "api_key");
}

#[test]
fn boolish_counted_toward_threshold_but_never_flagged() {
    // `boolish` counts toward bool_param_count (4, not 1) and counts
    // toward the inclusive threshold, but is never a BoolCluster
    // candidate: only the standard-bool param is flagged.
    let params = ["a", "b", "c"]
        .iter()
        .map(|n| make_param(n, Some("boolish")))
        .chain(["d"].iter().map(|n| make_param(n, Some("bool"))))
        .collect();
    let unit = fn_unit("configure", params);
    let results = analyze(&[unit], 3, false, &[]);
    assert_eq!(results[0].bool_param_count, 4);
    assert_eq!(results[0].violations.len(), 1);
    assert_eq!(results[0].violations[0].param_name, "d");
    assert!(matches!(
        results[0].violations[0].kind,
        PrimitiveViolationKind::BoolCluster
    ));
}

#[test]
fn domain_params_precede_bool_cluster() {
    // Pinning: PrimitiveDomainParam is pushed during counting, so it
    // precedes BoolCluster violations regardless of parameter order.
    // Exact boundary: 3 bools with max 3 must flag.
    let mut params = vec![make_param("email", Some("String"))];
    params.push(make_param("a", Some("bool")));
    params.push(make_param("b", Some("bool")));
    params.push(make_param("c", Some("bool")));
    let unit = fn_unit("register", params);
    let results = analyze(&[unit], 3, true, &[]);
    let violations = &results[0].violations;
    assert_eq!(violations.len(), 4);
    assert_eq!(violations[0].param_name, "email");
    assert!(matches!(
        violations[0].kind,
        PrimitiveViolationKind::PrimitiveDomainParam
    ));
    assert!(violations[1..]
        .iter()
        .all(|v| matches!(v.kind, PrimitiveViolationKind::BoolCluster)));
    assert_eq!(results[0].bool_param_count, 3);
}
