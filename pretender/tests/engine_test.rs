//! Query-engine capture-extraction tests: query scaffolding, function
//! extraction, and branch capture through the public API.
//!
//! # Responsibilities
//! Assert that compiled queries scaffold and that function units and
//! branch kinds (if/elif/loop/catch/logical/ternary) are captured with
//! correct names, kinds, exports, and nesting.
//!
//! # Rationale
//! Split from the former `#[cfg(test)]` module in `src/engine.rs`
//! (ratcheted in pretender-5zo.15.3) so each test target stays within
//! the file-lines cap; assertions are unchanged and `unwrap`/`expect`
//! became sanctioned `unwrap_or_else` panics in the move.

use std::path::Path;

use pretender::engine::QueryEngine;
use pretender::model::{Block, Branch, BranchKind, Language, Node, UnitKind};

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

// --- Phase 1: Query engine scaffolding ---

#[test]
fn compiles_valid_query() {
    let result = QueryEngine::new(python_language(), Language::Python, python_query_source());
    assert!(result.is_ok(), "valid query should compile");
}

#[test]
fn rejects_invalid_query() {
    let result = QueryEngine::new(
        python_language(),
        Language::Python,
        "(invalid_node_xxx) @bogus",
    );
    assert!(result.is_err(), "invalid query should fail");
}

// --- Phase 2: Function extraction via captures ---

#[test]
fn extracts_single_function() {
    let engine = engine();
    let source = "def hello(x):\n    return x\n";
    let (module, diags) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    assert!(diags.is_empty());
    assert_eq!(module.units.len(), 1);
    assert_eq!(module.units[0].name, "hello");
    assert_eq!(module.units[0].kind, UnitKind::Function);
    assert_eq!(module.units[0].span.start_line, 1);
    assert_eq!(module.units[0].span.end_line, 2);
}

#[test]
fn extracts_multiple_functions() {
    let engine = engine();
    let source = "def foo():\n    pass\n\ndef bar():\n    pass\n";
    let (module, _) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    assert_eq!(module.units.len(), 2);
    let names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(names, vec!["foo", "bar"]);
}

#[test]
fn extracts_function_parameters() {
    let engine = engine();
    let source = "def greet(name, age):\n    pass\n";
    let (module, _) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    assert_eq!(module.units[0].parameters.len(), 2);
    assert_eq!(module.units[0].parameters[0].name, "name");
    assert_eq!(module.units[0].parameters[1].name, "age");
}

#[test]
fn detects_class_methods() {
    let engine = engine();
    let source = "class Foo:\n    def bar(self):\n        pass\n";
    let (module, _) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    assert_eq!(module.units.len(), 1);
    assert_eq!(module.units[0].name, "bar");
    assert_eq!(module.units[0].kind, UnitKind::Method);
}

#[test]
fn detects_constructor() {
    let engine = engine();
    let source = "class Foo:\n    def __init__(self):\n        pass\n";
    let (module, _) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    assert_eq!(module.units[0].name, "__init__");
    assert_eq!(module.units[0].kind, UnitKind::Ctor);
}

#[test]
fn private_functions_not_exported() {
    let engine = engine();
    let source = "def _private():\n    pass\n\ndef public():\n    pass\n";
    let (module, _) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    assert!(!module.units[0].is_exported);
    assert!(module.units[1].is_exported);
}

// --- Phase 3: Branch extraction via captures ---

#[test]
fn captures_top_level_branch_kinds() {
    // Each source yields exactly one top-level branch of the expected
    // kind; table-driven so the shape varies from the recursive probes.
    let cases = [
        (
            "def f():\n    if True:\n        pass\n",
            BranchKind::If,
            "test.py",
        ),
        (
            "def f():\n    for i in range(10):\n        pass\n",
            BranchKind::Loop,
            "loop.py",
        ),
        (
            "def f():\n    while True:\n        pass\n",
            BranchKind::Loop,
            "while.py",
        ),
        (
            "def f(x):\n    return x if x > 0 else -x\n",
            BranchKind::Ternary,
            "ternary.py",
        ),
    ];
    for (source, kind, file) in cases {
        let engine = engine();
        let (module, _) = engine
            .parse(Path::new(file), source)
            .unwrap_or_else(|e| panic!("parse {file}: {e:?}"));

        let branches = collect_branches(&module.units[0].body);
        assert_eq!(branches.len(), 1, "{file}: one top-level branch");
        assert_eq!(branches[0].kind, kind, "{file}: captured branch kind");
    }
}

#[test]
fn captures_elif_branch() {
    let engine = engine();
    let source = "def f():\n    if True:\n        pass\n    elif False:\n        pass\n";
    let (module, _) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    let branches = collect_branches_recursive(&module.units[0].body);
    let kinds: Vec<BranchKind> = branches.iter().map(|b| b.kind).collect();
    assert!(kinds.contains(&BranchKind::If));
    assert!(kinds.contains(&BranchKind::ElseIf));
}

#[test]
fn captures_logical_operator_branches() {
    // `a and b` and `a or b` each capture the If condition plus the
    // logical operator branch; table-driven over the two operators.
    let cases = [
        (
            "def f(a, b):\n    if a and b:\n        pass\n",
            BranchKind::LogicalAnd,
            "and.py",
        ),
        (
            "def f(a, b):\n    if a or b:\n        pass\n",
            BranchKind::LogicalOr,
            "or.py",
        ),
    ];
    for (source, kind, file) in cases {
        let engine = engine();
        let (module, _) = engine
            .parse(Path::new(file), source)
            .unwrap_or_else(|e| panic!("parse {file}: {e:?}"));

        let branches = collect_branches_recursive(&module.units[0].body);
        let kinds: Vec<BranchKind> = branches.iter().map(|b| b.kind).collect();
        assert!(kinds.contains(&BranchKind::If), "{file}: if condition");
        assert!(kinds.contains(&kind), "{file}: logical operator kind");
    }
}

#[test]
fn captures_except_as_catch() {
    let engine = engine();
    let source = "def f():\n    try:\n        pass\n    except ValueError:\n        pass\n";
    let (module, _) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    let branches = collect_branches_recursive(&module.units[0].body);
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].kind, BranchKind::Catch);
}

#[test]
fn nested_branches_have_correct_nesting() {
    let engine = engine();
    let source = "def f():\n    if True:\n        if False:\n            pass\n";
    let (module, _) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    let branches = collect_branches_recursive(&module.units[0].body);
    assert_eq!(branches.len(), 2);
    assert_eq!(branches[0].nesting_at, 0);
    assert_eq!(branches[1].nesting_at, 1);
}

// --- Helpers ---

fn collect_branches(block: &Block) -> Vec<&Branch> {
    block
        .children
        .iter()
        .filter_map(|n| match n {
            Node::Branch(b) => Some(b),
            _ => None,
        })
        .collect()
}

fn collect_branches_recursive(block: &Block) -> Vec<&Branch> {
    let mut branches = Vec::new();
    for child in &block.children {
        match child {
            Node::Branch(b) => branches.push(b),
            Node::NestedBlock(b) => branches.extend(collect_branches_recursive(b)),
            _ => {}
        }
    }
    branches
}
