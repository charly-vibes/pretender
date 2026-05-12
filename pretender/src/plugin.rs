#![allow(dead_code)]

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub display_name: String,
    pub extensions: Vec<String>,
    #[serde(default)]
    pub shebangs: Vec<String>,
    pub tree_sitter: TreeSitterSource,
    pub query: String,
    #[serde(default)]
    pub branches: BTreeMap<String, BranchWeights>,
    #[serde(default)]
    pub assertions: Assertions,
    #[serde(default)]
    pub smell_weights: BTreeMap<String, f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TreeSitterSource {
    pub source: String,
    pub rev: String,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct BranchWeights {
    #[serde(default = "default_weight")]
    pub cyclomatic: u32,
    #[serde(default = "default_weight")]
    pub cognitive: u32,
}

fn default_weight() -> u32 {
    1
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Assertions {
    #[serde(default)]
    pub patterns: Vec<String>,
}

impl PluginManifest {
    pub fn from_toml(source: &str) -> Result<Self> {
        toml::from_str(source).context("failed to parse plugin manifest")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PYTHON_MANIFEST: &str = include_str!("../../languages/python/plugin.toml");

    #[test]
    fn parses_python_manifest() {
        let manifest = PluginManifest::from_toml(PYTHON_MANIFEST).expect("parses");
        assert_eq!(manifest.name, "python");
        assert_eq!(manifest.display_name, "Python");
        assert_eq!(manifest.extensions, vec![".py"]);
        assert_eq!(manifest.query, "metrics.scm");
    }

    #[test]
    fn python_manifest_has_branch_weights() {
        let manifest = PluginManifest::from_toml(PYTHON_MANIFEST).unwrap();
        let if_weight = manifest
            .branches
            .get("@branch.if")
            .expect("@branch.if entry present");
        assert_eq!(if_weight.cyclomatic, 1);
        assert_eq!(if_weight.cognitive, 1);
    }

    #[test]
    fn python_manifest_has_assertion_patterns() {
        let manifest = PluginManifest::from_toml(PYTHON_MANIFEST).unwrap();
        assert!(!manifest.assertions.patterns.is_empty());
        assert!(manifest
            .assertions
            .patterns
            .iter()
            .any(|p| p.contains("pytest")));
    }

    #[test]
    fn rejects_invalid_toml() {
        let err = PluginManifest::from_toml("not = valid =").unwrap_err();
        assert!(err.to_string().contains("failed to parse plugin manifest"));
    }

    #[test]
    fn missing_required_field_errors() {
        let toml = r#"
name = "x"
display_name = "X"
extensions = [".x"]
"#;
        assert!(PluginManifest::from_toml(toml).is_err());
    }
}
