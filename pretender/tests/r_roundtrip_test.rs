//! R parser integration tests.
//!
//! # Responsibilities
//! Pin R-specific parse behavior through the shared engine.
//!
//! # Rationale
//! The round-trip unit tests moved out of the `#[cfg(test)]` module in
//! `src/r.rs` (pretender-948q registry migration): they pin the R adapter's
//! zero-capture contract, and this is their matching integration target.
//! The query is included from the adapter's data directory so the pin rides
//! the same bytes the production registry uses.

use pretender::engine::QueryEngine;
use pretender::model::{Diagnostic, Language, Module};
use std::path::Path;

fn r_engine() -> QueryEngine {
    QueryEngine::new(
        tree_sitter_r::LANGUAGE.into(),
        Language::R,
        include_str!("../languages/r/metrics.scm"),
    )
    .unwrap_or_else(|e| panic!("r query engine: {e:?}"))
}

const CLASS_SYSTEMS_SOURCE: &str = r#"person <- structure(list(name = "a", age = 1), class = "Person")
setClass("Slot", slots = c(x = "numeric"))
klass <- R6Class("Cell", public = list(held = NULL))
name_of <- function(p) p$name
"#;

/// The zero-capture contract: a clean parse with an empty
/// `type_definitions` list.
fn assert_zero_type_captures(module: &Module, diags: &[Diagnostic]) {
    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
    assert!(
        module.type_definitions.is_empty(),
        "function-based R class systems must not be captured, got: {:?}",
        module.type_definitions
    );
}

#[test]
fn r_s3_s4_r6_class_systems_not_captured_zero_type_captures() {
    // Zero-capture decision (tasks 6.6, design D3): R's class systems
    // are function-based — `structure(..., class = "x")` / `setClass` /
    // `R6Class` are plain call expressions in the grammar, with no
    // type-definition node to anchor `@type.definition` on. Per D3
    // the adapter ships with zero type captures; parsing must stay
    // valid with an empty `type_definitions` list.
    let engine = r_engine();
    let (module, diags) = engine
        .parse(Path::new("r_sample.R"), CLASS_SYSTEMS_SOURCE)
        .unwrap_or_else(|e| panic!("r_sample.R must parse: {e:?}"));
    assert_zero_type_captures(&module, &diags);
    assert_eq!(module.units.len(), 1);
    assert_eq!(module.units[0].name, "name_of");
}
