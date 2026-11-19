use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const R_QUERY: &str = include_str!("../languages/r/metrics.scm");
const R_MANIFEST: &str = include_str!("../languages/r/plugin.toml");

pub struct RParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(R_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl RParser {
    fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_r::LANGUAGE.into(),
            Language::R,
            R_QUERY,
            &m.branches,
            &m.smell_weights,
        )
    }
}

impl Parser for RParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::QueryEngine;
    use std::path::Path;

    fn r_engine() -> QueryEngine {
        QueryEngine::new(tree_sitter_r::LANGUAGE.into(), Language::R, R_QUERY).unwrap()
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
        let source = r#"person <- structure(list(name = "a", age = 1), class = "Person")
setClass("Slot", slots = c(x = "numeric"))
klass <- R6Class("Cell", public = list(held = NULL))
name_of <- function(p) p$name
"#;
        let (module, diags) = engine.parse(Path::new("r_sample.R"), source).unwrap();

        assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
        assert!(
            module.type_definitions.is_empty(),
            "function-based R class systems must not be captured, got: {:?}",
            module.type_definitions
        );
        assert_eq!(module.units.len(), 1);
        assert_eq!(module.units[0].name, "name_of");
    }
}
