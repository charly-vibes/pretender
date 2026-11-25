//! Query-engine fixture and metric-application tests: real fixture
//! parsing, expected cyclomatic scores, ABC capture, weight overrides,
//! and parse-error handling through the public API.
//!
//! # Responsibilities
//! Assert fixture-driven behavior: python_simple.py parses without
//! diagnostics and yields the expected units and cyclomatic scores,
//! assignment+call captures feed ABC, branch-weight overrides change
//! cognitive scores, and parse errors skip the file with diagnostics.
//!
//! # Rationale
//! Split from the former `#[cfg(test)]` module in `src/engine.rs`
//! (ratcheted in pretender-5zo.15.3) so each test target stays within
//! the file-lines cap; assertions are unchanged and `unwrap`/`expect`
//! became sanctioned `unwrap_or_else` panics in the move.

use std::collections::BTreeMap;
use std::path::Path;

use pretender::engine::{QueryEngine, WeightMaps};
use pretender::metrics::{abc, cognitive, CyclomaticComplexity};
use pretender::model::*;
use pretender::plugin::BranchWeights;
use pretender::plugin::NestingConfig;

fn python_language() -> tree_sitter::Language {
    tree_sitter_python::LANGUAGE.into()
}

fn python_query_source() -> &'static str {
    include_str!("../languages/python/metrics.scm")
}

fn engine() -> QueryEngine {
    QueryEngine::new(python_language(), Language::Python, python_query_source())
        .unwrap_or_else(|e| panic!("python engine: {e:?}"))
}

fn weighted_engine(weights: BTreeMap<String, BranchWeights>) -> QueryEngine {
    let empty_calls = BTreeMap::new();
    QueryEngine::new_with_branch_weights(
        python_language(),
        Language::Python,
        python_query_source(),
        &WeightMaps {
            branch: &weights,
            call: &empty_calls,
            nesting: &NestingConfig::default(),
        },
    )
    .unwrap_or_else(|e| panic!("engine: {e:?}"))
}

#[test]
fn custom_branch_weights_override_cognitive_scores() {
    let engine = weighted_engine(BTreeMap::from([(
        "@branch.if".to_string(),
        BranchWeights {
            cyclomatic: 1,
            cognitive: 3,
        },
    )]));
    let source = "def f():\n    if True:\n        pass\n";
    let (module, _) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    assert_eq!(cognitive(&module.units[0]), 3);
}

// --- Phase 4: Full fixture integration ---

#[test]
fn parse_python_fixture() {
    let engine = engine();
    let fixture = include_str!("fixtures/python_simple.py");
    let (module, diags) = engine
        .parse(Path::new("python_simple.py"), fixture)
        .unwrap_or_else(|e| panic!("parse python_simple.py: {e:?}"));

    assert!(diags.is_empty(), "fixture should parse without diagnostics");
    assert!(
        module.units.len() >= 2,
        "fixture must produce at least 2 CodeUnits, got {}",
        module.units.len()
    );

    let names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
    assert!(names.contains(&"simple"));
    assert!(names.contains(&"with_branch"));
    assert!(names.contains(&"complex_func"));
    assert!(names.contains(&"__init__"));
    assert!(names.contains(&"method_simple"));
    assert!(names.contains(&"method_with_loop"));
}

#[test]
fn fixture_metrics_match_expected() {
    let engine = engine();
    let fixture = include_str!("fixtures/python_simple.py");
    let (module, _) = engine
        .parse(Path::new("python_simple.py"), fixture)
        .unwrap_or_else(|e| panic!("parse python_simple.py: {e:?}"));

    let cc = CyclomaticComplexity;
    let expected = vec![
        ("simple", 1),
        ("with_branch", 2),
        ("complex_func", 6),
        ("__init__", 1),
        ("method_simple", 1),
        ("method_with_loop", 3),
    ];

    for (name, expected_cc) in expected {
        let unit = module
            .units
            .iter()
            .find(|u| u.name == name)
            .unwrap_or_else(|| panic!("missing CodeUnit: {name}"));
        let actual = cc.calculate(unit);
        assert_eq!(
            actual, expected_cc,
            "{name}: expected CC={expected_cc}, got CC={actual}"
        );
    }
}

// --- ABC: assignments + calls are captured ---

#[test]
fn captures_assignments_and_calls_for_abc() {
    let engine = engine();
    let source = "def f(x):\n    y = x + 1\n    z = foo(y)\n    return bar(z)\n";
    let (module, _) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    let unit = &module.units[0];
    let abc_score = abc(unit);
    assert!(
        abc_score > 0.0,
        "ABC must reflect assignment + call captures; got {abc_score}",
    );

    let mut counts = AbcCounts::default();
    collect_assigns_and_calls(&unit.body, &mut counts);
    assert!(
        counts.assigns >= 2,
        "expected ≥2 assignments, got {}",
        counts.assigns
    );
    assert!(counts.calls >= 2, "expected ≥2 calls, got {}", counts.calls);
}

/// Running assignment/call tallies for the ABC capture probe above.
#[derive(Default)]
struct AbcCounts {
    assigns: u32,
    calls: u32,
}

fn collect_assigns_and_calls(block: &Block, counts: &mut AbcCounts) {
    for child in &block.children {
        collect_child(child, counts);
    }
}

fn collect_child(child: &Node, counts: &mut AbcCounts) {
    if let Node::NestedBlock(b) = child {
        collect_assigns_and_calls(b, counts);
    }
    bump_assigns(child, counts);
    bump_calls(child, counts);
}

fn bump_assigns(child: &Node, counts: &mut AbcCounts) {
    if matches!(child, Node::Assignment(_)) {
        counts.assigns += 1;
    }
}

fn bump_calls(child: &Node, counts: &mut AbcCounts) {
    if matches!(child, Node::Call(_)) {
        counts.calls += 1;
    }
}

// --- Phase 5: Error handling ---

#[test]
fn parse_error_emits_diagnostic_and_skips() {
    let engine = engine();
    let bad_source = "def f(\n    # unclosed paren, malformed\n";
    let (module, diags) = engine
        .parse(Path::new("bad.py"), bad_source)
        .unwrap_or_else(|e| panic!("parse bad.py: {e:?}"));

    assert!(!diags.is_empty(), "should emit at least one diagnostic");
    assert!(
        module.units.is_empty(),
        "should skip file with parse errors (no partial Module)"
    );
}
