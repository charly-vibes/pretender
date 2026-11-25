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
use pretender::model::Language;
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
