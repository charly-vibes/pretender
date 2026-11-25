//! Query-engine assertion-counting and export-visibility tests through
//! the public API.
//!
//! # Responsibilities
//! Assert per-unit assertion counting (Rust/Julia test macros count,
//! non-test macros do not, nested definitions do not leak to parents),
//! underscore-prefix export rules, default call weights, and that
//! unregistered macro calls stay uncounted.
//!
//! # Rationale
//! Split from the former `#[cfg(test)]` module in `src/engine.rs`
//! (ratcheted in pretender-5zo.15.3) so each test target stays within
//! the file-lines cap; assertions are unchanged and `unwrap`/`expect`
//! became sanctioned `unwrap_or_else` panics in the move.

use std::path::Path;

use pretender::engine::QueryEngine;
use pretender::model::*;

fn rust_engine() -> QueryEngine {
    QueryEngine::new(
        tree_sitter_rust::LANGUAGE.into(),
        Language::Rust,
        include_str!("../languages/rust/metrics.scm"),
    )
    .unwrap_or_else(|e| panic!("engine: {e:?}"))
}

fn julia_engine() -> QueryEngine {
    QueryEngine::new(
        tree_sitter_julia::LANGUAGE.into(),
        Language::Julia,
        include_str!("../languages/julia/metrics.scm"),
    )
    .unwrap_or_else(|e| panic!("engine: {e:?}"))
}

#[test]
fn rust_assertion_macros_count_as_assertions() {
    let engine = rust_engine();
    let source = "#[test]\nfn adds() {\n    let a = 2;\n    assert_eq!(a, 2);\n    assert!(a > 0);\n    assert_ne!(a, 3);\n    debug_assert_eq!(a, 2);\n}\n";
    let (module, _) = engine
        .parse(Path::new("test.rs"), source)
        .unwrap_or_else(|e| panic!("parse test.rs: {e:?}"));
    assert_eq!(module.units[0].assertions, 4);
}

#[test]
fn rust_non_assertion_macros_do_not_count() {
    let engine = rust_engine();
    let source = "#[test]\nfn formats() {\n    let s = format!(\"{}\", 1);\n    vec![1, 2];\n    panic_if_bad(s);\n}\n\nfn panic_if_bad<T>(_: T) {}\n";
    let (module, _) = engine
        .parse(Path::new("test.rs"), source)
        .unwrap_or_else(|e| panic!("parse test.rs: {e:?}"));
    let unit = module
        .units
        .iter()
        .find(|u| u.name == "formats")
        .unwrap_or_else(|| panic!("unit formats captured"));
    assert_eq!(unit.assertions, 0);
}

#[test]
fn julia_test_macros_count_as_assertions() {
    let engine = julia_engine();
    let source = "function check(x)\n    @test x > 0\n    @test_throws DomainError sqrt(-1)\n    @test_broken x < 0\n    @test_logs (:info) log_it(x)\n    @assert x > 0\n    return x\nend\n";
    let (module, _) = engine
        .parse(Path::new("test.jl"), source)
        .unwrap_or_else(|e| panic!("parse test.jl: {e:?}"));
    assert_eq!(module.units[0].assertions, 5);
}

#[test]
fn julia_non_test_macros_do_not_count() {
    let engine = julia_engine();
    let source = "function show_it(x)\n    @show x\n    @info \"hello\" x\n    return x\nend\n";
    let (module, _) = engine
        .parse(Path::new("test.jl"), source)
        .unwrap_or_else(|e| panic!("parse test.jl: {e:?}"));
    assert_eq!(module.units[0].assertions, 0);
}

#[test]
fn underscore_prefixed_unit_is_not_exported() {
    let engine = rust_engine();
    let source = "fn _helper() {}\n\nfn helper() {}\n";
    let (module, _) = engine
        .parse(Path::new("test.rs"), source)
        .unwrap_or_else(|e| panic!("parse test.rs: {e:?}"));
    let hidden = module
        .units
        .iter()
        .find(|u| u.name == "_helper")
        .unwrap_or_else(|| panic!("unit _helper captured"));
    let public = module
        .units
        .iter()
        .find(|u| u.name == "helper")
        .unwrap_or_else(|| panic!("unit helper captured"));
    assert!(!hidden.is_exported);
    assert!(public.is_exported);
}

#[test]
fn parsed_call_site_gets_default_weight() {
    let engine = rust_engine();
    let source = "fn calls_helper() {\n    helper(1);\n}\n";
    let (module, _) = engine
        .parse(Path::new("test.rs"), source)
        .unwrap_or_else(|e| panic!("parse test.rs: {e:?}"));
    let unit = module
        .units
        .iter()
        .find(|u| u.name == "calls_helper")
        .unwrap_or_else(|| panic!("unit calls_helper captured"));
    let call = unit
        .body
        .children
        .iter()
        .find_map(|n| match n {
            Node::Call(c) if c.callee == "helper" => Some(c),
            _ => None,
        })
        .unwrap_or_else(|| panic!("call site captured"));
    assert_eq!(call.smell_weight, 1.0);
}

#[test]
fn counts_assertions_per_code_unit() {
    let engine = engine_py();
    let source = "def test_assertions():\n    assert ok\n    self.assertEqual(a, b)\n    pytest.raises(ValueError)\n";
    let (module, _) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    assert_eq!(module.units[0].assertions, 3);
}

#[test]
fn nested_function_assertions_do_not_count_toward_parent() {
    let engine = engine_py();
    let source = "def test_outer():\n    def helper():\n        assert True\n    helper()\n";
    let (module, _) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    let outer = module
        .units
        .iter()
        .find(|unit| unit.name == "test_outer")
        .unwrap_or_else(|| panic!("unit test_outer captured"));
    let inner = module
        .units
        .iter()
        .find(|unit| unit.name == "helper")
        .unwrap_or_else(|| panic!("unit helper captured"));
    assert_eq!(outer.assertions, 0);
    assert_eq!(inner.assertions, 1);
}

fn engine_py() -> QueryEngine {
    QueryEngine::new(
        tree_sitter_python::LANGUAGE.into(),
        Language::Python,
        include_str!("../languages/python/metrics.scm"),
    )
    .unwrap_or_else(|e| panic!("engine: {e:?}"))
}
