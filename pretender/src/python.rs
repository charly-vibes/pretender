use anyhow::Result;
use std::path::Path;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const PYTHON_MANIFEST: &str = include_str!("../../languages/python/plugin.toml");
const PYTHON_QUERY: &str = include_str!("../../languages/python/metrics.scm");

pub struct PythonParser;

impl PythonParser {
    pub fn manifest() -> Result<PluginManifest> {
        PluginManifest::from_toml(PYTHON_MANIFEST)
    }

    fn engine(&self) -> Result<QueryEngine> {
        QueryEngine::new(
            tree_sitter_python::LANGUAGE.into(),
            Language::Python,
            PYTHON_QUERY,
        )
    }
}

impl Parser for PythonParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }

    fn extensions(&self) -> &[&str] {
        &["py"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_loads_and_advertises_py_extension() {
        let manifest = PythonParser::manifest().expect("manifest parses");
        assert_eq!(manifest.name, "python");
        assert!(manifest.extensions.iter().any(|e| e == ".py"));
    }

    #[test]
    fn manifest_query_filename_matches_embedded_query() {
        let manifest = PythonParser::manifest().unwrap();
        assert_eq!(manifest.query, "metrics.scm");
    }
}
