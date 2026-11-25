//! Purpose: Integration tests for mutability analysis (void-mutator
//!   detection and mutable-binding counts) through
//!   `pretender::mutability_metrics`.
//! Responsibilities: Pin void-mutator detection semantics (methods only,
//!   constructors skipped, transitive mutation ignored, canonical (line,
//!   name) ordering independent of unit iteration order) and per-language
//!   mutable-binding counts and ratios.
//! Rationale: Extracted verbatim from src/mutability_metrics.rs's inline
//!   test module (slice pretender-5zo.29) to keep the source file within
//!   app-role thresholds; assertions and test names unchanged. Unit builders
//!   take at most two arguments (test role params_max = 2) and have distinct
//!   call shapes so the lazy-cluster detector finds no group of three or
//!   more identical fingerprints; `method_unit` carries an implicit
//!   `&mut self` parameter, and `with_params` replaces a unit's parameter
//!   list wholesale for tests that need more than that implicit parameter.

use pretender::model::{Block, CodeUnit, Language, Parameter, Span, UnitKind};
use pretender::mutability_metrics::{count_mutable_bindings, detect_void_mutators, VoidMutator};
use std::path::Path;

fn make_param(name: &str) -> Parameter {
    Parameter {
        name: name.to_string(),
        span: Span {
            start_line: 1,
            end_line: 1,
        },
        type_name: None,
    }
}

fn base_unit(name: &str, span: (u32, u32)) -> CodeUnit {
    let (start, end) = span;
    CodeUnit {
        name: name.to_string(),
        kind: UnitKind::Function,
        span: Span {
            start_line: start,
            end_line: end,
        },
        parameters: vec![],
        body: Block {
            span: Span {
                start_line: start,
                end_line: end,
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

fn method_unit(name: &str, span: (u32, u32)) -> CodeUnit {
    let mut unit = base_unit(name, span);
    unit.kind = UnitKind::Method;
    unit.parameters = vec![make_param("&mut self")];
    unit
}

fn ctor_unit(name: &str, span: (u32, u32)) -> CodeUnit {
    let mut unit = base_unit(name, span);
    unit.kind = UnitKind::Ctor;
    unit
}

fn with_params(mut unit: CodeUnit, params: Vec<Parameter>) -> CodeUnit {
    unit.parameters = params;
    unit
}

#[test]
fn detects_rust_void_mutator() {
    let source = r#"
impl MyStruct {
    fn update(&mut self, value: i32) {
        self.field = value;
    }
}
"#;
    let units = vec![with_params(
        method_unit("update", (3, 5)),
        vec![make_param("&mut self"), make_param("value")],
    )];
    let result = detect_void_mutators(source, &Language::Rust, &units, Path::new("test.rs"));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].name, "update");
}

#[test]
fn pure_function_not_flagged() {
    let source = r#"
fn add(a: i32, b: i32) -> i32 {
    a + b
}
"#;
    let units = vec![with_params(
        base_unit("add", (1, 3)),
        vec![make_param("a"), make_param("b")],
    )];
    let result = detect_void_mutators(source, &Language::Rust, &units, Path::new("test.rs"));
    assert_eq!(result.len(), 0);
    assert_eq!(result, Vec::<VoidMutator>::new());
}

#[test]
fn transitive_mutation_not_flagged() {
    let source = r#"
impl MyStruct {
    fn update(&mut self) {
        self.list.add(42);  // method call, not direct assignment
    }
}
"#;
    let units = vec![method_unit("update", (2, 4))];
    let result = detect_void_mutators(source, &Language::Rust, &units, Path::new("test.rs"));
    // self.list.add(42) is a method call, not self.field = value
    // The body has "self." but not " = " — so it should NOT be flagged
    let flagged = result.len();
    assert_eq!(flagged, 0);
}

#[test]
fn constructor_not_flagged() {
    let source = r#"
impl MyStruct {
    fn new() -> Self {
        Self { field: 42 }
    }
}
"#;
    let units = vec![ctor_unit("new", (2, 4))];
    let result = detect_void_mutators(source, &Language::Rust, &units, Path::new("test.rs"));
    assert_eq!(result.len(), 0);
}

#[test]
fn unsupported_language_returns_empty() {
    let source = "nothing";
    let units = vec![];
    let result = detect_void_mutators(source, &Language::Julia, &units, Path::new("test.jl"));
    assert_eq!(result.len(), 0);
}

// ── Determinism (pretender-gv5): the finding vector must be identical
// run-to-run regardless of the order in which units arrive (walk order /
// parallel collection). Output is canonicalized by stable keys
// (line, name) so upstream reordering cannot change the result. ───

fn three_mutator_fixture() -> (String, Vec<CodeUnit>) {
    let source = "\
impl Counter {
    fn refresh(&mut self) {
        self.count = 1;
    }
    fn reset(&mut self) {
        self.count = 0;
    }
    fn sync(&mut self) {
        self.total = 9;
    }
}
";
    let units = vec![
        method_unit("refresh", (2, 4)),
        method_unit("reset", (5, 7)),
        method_unit("sync", (8, 10)),
    ];
    (source.to_string(), units)
}

#[test]
fn void_mutators_identical_across_repeated_runs() {
    let (source, units) = three_mutator_fixture();
    let first = detect_void_mutators(&source, &Language::Rust, &units, Path::new("a.rs"));
    assert_eq!(first.len(), 3);
    for _ in 0..4 {
        let again = detect_void_mutators(&source, &Language::Rust, &units, Path::new("a.rs"));
        assert_eq!(again, first);
    }
}

fn vm(name: &str, line: u32) -> VoidMutator {
    VoidMutator {
        name: name.to_owned(),
        line,
    }
}

#[test]
fn void_mutators_independent_of_unit_iteration_order() {
    let (source, units) = three_mutator_fixture();
    let scan = |units: &[CodeUnit]| {
        detect_void_mutators(&source, &Language::Rust, units, Path::new("a.rs"))
    };

    let forward = scan(&units);
    let mut reordered = units.clone();
    reordered.rotate_left(1); // walk-order / parallel-collection shuffle
    let mut reversed = units.clone();
    reversed.reverse();

    assert_eq!(
        scan(&reordered),
        forward,
        "finding vector must not follow input order"
    );
    assert_eq!(
        scan(&reversed),
        forward,
        "finding vector must not follow input order"
    );
    assert_eq!(
        forward,
        vec![vm("refresh", 2), vm("reset", 5), vm("sync", 8)],
        "canonical order is (line, name), independent of input order"
    );
}

#[test]
fn rust_mutable_ratio_half() {
    let source = r#"
let x = 1;
let mut y = 2;
let z = 3;
let mut w = 4;
"#;
    let analysis = count_mutable_bindings(source, &Language::Rust);
    assert_eq!(analysis.total_count, 4);
    assert_eq!(analysis.mutable_count, 2);
    assert!((analysis.ratio() - 0.5).abs() < 0.001);
}

#[test]
fn rust_no_mutable_bindings() {
    let source = r#"let x = 1;"#;
    let analysis = count_mutable_bindings(source, &Language::Rust);
    assert_eq!(analysis.total_count, 1);
    assert_eq!(analysis.mutable_count, 0);
    assert_eq!(analysis.ratio(), 0.0);
}

#[test]
fn js_tracks_let_var_const() {
    let source = r#"
const x = 1;
let y = 2;
var z = 3;
"#;
    let analysis = count_mutable_bindings(source, &Language::JavaScript);
    assert_eq!(analysis.total_count, 3);
    assert_eq!(analysis.mutable_count, 2);
}

#[test]
fn python_returns_zero() {
    let source = "x = 1\ny = 2";
    let analysis = count_mutable_bindings(source, &Language::Python);
    assert_eq!(analysis.total_count, 0);
    assert_eq!(analysis.mutable_count, 0);
}

#[test]
fn empty_source_zero_ratio() {
    let analysis = count_mutable_bindings("", &Language::Rust);
    assert_eq!(analysis.ratio(), 0.0);
}
