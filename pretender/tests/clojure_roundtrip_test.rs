//! Clojure parser integration tests.
//!
//! # Responsibilities
//! Pin Clojure-specific parse behavior through the shared engine.
//!
//! # Rationale
//! The round-trip unit tests moved out of the `#[cfg(test)]` module in
//! `src/clojure.rs` (pretender-948q registry migration): they pin the
//! Clojure adapter's zero-capture contract, and this is their matching
//! integration target. The query is included from the adapter's data
//! directory so the pin rides the same bytes the production registry uses.

use pretender::engine::QueryEngine;
use pretender::model::{CodeUnit, Language, Module};
use std::path::Path;

fn clojure_engine() -> QueryEngine {
    QueryEngine::new(
        tree_sitter_clojure::LANGUAGE.into(),
        Language::Clojure,
        include_str!("../languages/clojure/metrics.scm"),
    )
    .unwrap_or_else(|e| panic!("clojure query engine: {e:?}"))
}

const DEFRECORD_DEFTYPE_SOURCE: &str =
    "(defrecord Point [x y])\n\n(deftype Cell [held]\n  (proto)\n  (area [this] 1))\n";

/// The zero-capture contract: a clean parse with an empty
/// `type_definitions` list.
#[test]
fn clojure_defrecord_deftype_ship_zero_type_captures() {
    // Zero-capture decision (tasks 6.6, design D2/D3): the pinned
    // grammar tree-sitter-clojure 0.1.0 exposes only generic
    // `list_lit`/`sym_lit` nodes — there is no distinct defrecord or
    // deftype node to anchor `@type.definition` on, and capturing
    // `list_lit` would map to `Class`, never the `Record` kind D3
    // reserves for defrecord. Per D2/D3 the adapter ships with zero
    // type captures; parsing must stay valid (spec: "Adapter without
    // type captures stays valid").
    let engine = clojure_engine();
    let (module, diags) = engine
        .parse(
            Path::new("clojure_types_sample.clj"),
            DEFRECORD_DEFTYPE_SOURCE,
        )
        .unwrap_or_else(|e| panic!("clojure sample must parse: {e:?}"));
    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
    assert!(
        module.type_definitions.is_empty(),
        "pinned grammar has no distinct type nodes; captures must stay empty, got: {:?}",
        module.type_definitions
    );
}

fn find_unit<'a>(module: &'a Module, name: &str) -> &'a CodeUnit {
    module
        .units
        .iter()
        .find(|u| u.name == name)
        .unwrap_or_else(|| panic!("{name} not captured"))
}

fn param_names(unit: &CodeUnit) -> Vec<&str> {
    unit.parameters.iter().map(|p| p.name.as_str()).collect()
}

fn parse_clojure_source(source: &'static str) -> Module {
    let (module, diags) = clojure_engine()
        .parse(Path::new("clojure_params.clj"), source)
        .unwrap_or_else(|e| panic!("clojure source must parse: {e:?}"));
    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
    module
}

/// Red gate (1v8k.6): a 3-parameter defn reports params: 3.
#[test]
fn clojure_single_arity_defn_names_params() {
    let module = parse_clojure_source(include_str!("fixtures/clojure_sample.clj"));
    let unit = find_unit(&module, "complex-func");
    assert_eq!(
        param_names(unit),
        vec!["a", "b", "items"],
        "arity vector symbols become parameters in declaration order"
    );
}

/// Variadic and destructuring shapes follow the documented rule:
/// `&` marks the variadic boundary (not a parameter); each destructuring
/// binding form counts as one parameter named by its source text.
#[test]
fn clojure_variadic_and_destructuring_bind_shapes() {
    let module = parse_clojure_source(include_str!("fixtures/clojure_params_sample.clj"));
    assert_eq!(
        param_names(find_unit(&module, "variadic")),
        vec!["a", "rest"],
        "& rest counts as one parameter, matching Python's *rest"
    );
    assert_eq!(
        param_names(find_unit(&module, "deep-bind")),
        vec!["a", "[b c]"],
        "one parameter per destructuring binding, not per inner symbol"
    );
    assert_eq!(
        param_names(find_unit(&module, "destr-map")),
        vec!["x", "{:keys [a b]}"],
        "map bindings count as one parameter"
    );
}
