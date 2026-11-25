use super::*;
use crate::model::{Import, Language, Span};

fn make_module(path: &str, imports: Vec<&str>) -> (PathBuf, Module) {
    let path = PathBuf::from(path);
    let module = Module {
        path: path.clone(),
        language: Language::Rust,
        span: Span {
            start_line: 1,
            end_line: 10,
        },
        lines_total: 10,
        lines_code: 8,
        lines_comment: 2,
        units: vec![],
        type_definitions: Vec::new(),
        imports: imports.into_iter().map(make_import).collect(),
    };
    (path, module)
}

/// One default-span import capture for the fixtures.
fn make_import(module: &str) -> Import {
    Import {
        module: module.to_string(),
        name: None,
        alias: None,
        span: Span {
            start_line: 1,
            end_line: 1,
        },
    }
}

fn make_thresholds(counts: (u32, u32, u32, u32), cycles: bool) -> CouplingThresholds {
    let (ce, ca, cbo, lcom) = counts;
    CouplingThresholds {
        ce_max: ce,
        ca_max: ca,
        cbo_max: cbo,
        lcom_hs_max: lcom,
        cycle_detection: cycles,
    }
}

#[test]
fn no_thresholds_no_analysis() {
    let (p, m) = make_module("src/main.rs", vec!["std::collections::HashMap"]);
    let modules = vec![(p, &m)];
    let result = analyze(&modules, &make_thresholds((0, 0, 0, 0), false));
    assert!(result.modules.is_empty());
    assert!(result.cycles.is_empty());
}

#[test]
fn ce_threshold_violation() {
    // Module A imports B and C (Ce=2), ce_max=1
    let (ap, a) = make_module("src/a.rs", vec!["src/b.rs", "src/c.rs"]);
    let (bp, b) = make_module("src/b.rs", vec![]);
    let (cp, c) = make_module("src/c.rs", vec![]);
    let modules = vec![(ap, &a), (bp, &b), (cp, &c)];
    let result = analyze(&modules, &make_thresholds((1, 0, 0, 0), false));

    let a_report = result
        .modules
        .iter()
        .find(|r| r.path.contains("a.rs"))
        .unwrap();
    assert_eq!(a_report.violations.len(), 1);
    assert_eq!(a_report.violations[0].metric, "ce");
    assert_eq!(a_report.violations[0].actual, 2.0);
}

#[test]
fn no_cycle_in_acyclic_graph() {
    let (ap, a) = make_module("src/a.rs", vec!["src/b.rs"]);
    let (bp, b) = make_module("src/b.rs", vec!["src/c.rs"]);
    let (cp, c) = make_module("src/c.rs", vec![]);
    let modules = vec![(ap, &a), (bp, &b), (cp, &c)];
    let result = analyze(&modules, &make_thresholds((0, 0, 0, 0), true));
    assert!(result.cycles.is_empty());
}

#[test]
fn simple_cycle_detected() {
    let (ap, a) = make_module("src/a.rs", vec!["src/b.rs"]);
    let (bp, b) = make_module("src/b.rs", vec!["src/c.rs"]);
    let (cp, c) = make_module("src/c.rs", vec!["src/a.rs"]);
    let modules = vec![(ap, &a), (bp, &b), (cp, &c)];
    let result = analyze(&modules, &make_thresholds((0, 0, 0, 0), true));
    assert_eq!(result.cycles.len(), 1);
    assert_eq!(result.cycles[0].participants.len(), 3);
}

#[test]
fn std_imports_excluded() {
    let (p, a) = make_module("src/a.rs", vec!["std::collections::HashMap", "core::fmt"]);
    let modules = vec![(p, &a)];
    let result = analyze(&modules, &make_thresholds((1, 0, 0, 0), false));
    assert!(result.modules.is_empty() || result.modules.iter().all(|r| r.violations.is_empty()));
}

#[test]
fn ca_threshold_violation() {
    // Module B is imported by A and C (Ca=2), ca_max=1
    let (ap, a) = make_module("src/a.rs", vec!["src/b.rs"]);
    let (bp, b) = make_module("src/b.rs", vec![]);
    let (cp, c) = make_module("src/c.rs", vec!["src/b.rs"]);
    let modules = vec![(ap, &a), (bp, &b), (cp, &c)];
    let result = analyze(&modules, &make_thresholds((0, 1, 0, 0), false));

    let b_report = result
        .modules
        .iter()
        .find(|r| r.path.contains("b.rs"))
        .unwrap();
    assert_eq!(b_report.violations.len(), 1);
    assert_eq!(b_report.violations[0].metric, "ca");
    assert_eq!(b_report.violations[0].actual, 2.0);
}
