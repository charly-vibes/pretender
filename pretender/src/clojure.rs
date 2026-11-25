use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const CLOJURE_QUERY: &str = include_str!("../languages/clojure/metrics.scm");
const CLOJURE_MANIFEST: &str = include_str!("../languages/clojure/plugin.toml");

pub struct ClojureParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(CLOJURE_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl ClojureParser {
    fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_clojure::LANGUAGE.into(),
            Language::Clojure,
            CLOJURE_QUERY,
            &crate::engine::WeightMaps {
                branch: &m.branches,
                call: &m.smell_weights,
            },
        )
    }
}

impl Parser for ClojureParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::QueryEngine;
    use std::path::Path;

    fn clojure_engine() -> QueryEngine {
        QueryEngine::new(
            tree_sitter_clojure::LANGUAGE.into(),
            Language::Clojure,
            CLOJURE_QUERY,
        )
        .unwrap_or_else(|e| panic!("clojure query engine: {e:?}"))
    }

    const DEFRECORD_DEFTYPE_SOURCE: &str =
        "(defrecord Point [x y])\n\n(deftype Cell [held]\n  (proto)\n  (area [this] 1))\n";

    /// The zero-capture contract: a clean parse with an empty
    /// `type_definitions` list.
    fn assert_zero_type_captures(module: &Module, diags: &[Diagnostic]) {
        assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
        assert!(
            module.type_definitions.is_empty(),
            "pinned grammar has no distinct type nodes; captures must stay empty, got: {:?}",
            module.type_definitions
        );
    }

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
        assert_zero_type_captures(&module, &diags);
    }
}
