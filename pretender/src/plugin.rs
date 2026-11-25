// Load-bearing since the registry migration (948q): `LangRecord::manifest`
// parses these manifests in production and `WeightMaps` reads branches and
// smell_weights on every parse. The contract is pinned by
// `tests/tooling_test/plugins_test.rs::every_builtin_manifest_parses_and_matches_the_registry`.

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
    /// Flattened so every metric table stays top-level in `plugin.toml`.
    #[serde(flatten, default)]
    pub metric_sections: MetricSections,
}

/// Adapter metric weight/behavior sections: branch weights, assertion
/// capture config, per-callee smell weights, and the nesting-walk flag.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct MetricSections {
    #[serde(default)]
    pub branches: BTreeMap<String, BranchWeights>,
    #[serde(default)]
    pub assertions: Assertions,
    #[serde(default)]
    pub smell_weights: BTreeMap<String, f64>,
    #[serde(default)]
    pub nesting: NestingConfig,
}

/// Adapter nesting-walk configuration (proposal D4).
#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
pub struct NestingConfig {
    /// Branch-bearing forms wrap their body forms one level deeper
    /// (Clojure's list forms have no grammar-level `block` node).
    #[serde(default)]
    pub wrap_branch_bodies: bool,
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

    const PYTHON_MANIFEST: &str = include_str!("../languages/python/plugin.toml");

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
            .metric_sections
            .branches
            .get("@branch.if")
            .expect("@branch.if entry present");
        assert_eq!(if_weight.cyclomatic, 1);
        assert_eq!(if_weight.cognitive, 1);
    }

    #[test]
    fn python_manifest_has_assertion_patterns() {
        let manifest = PluginManifest::from_toml(PYTHON_MANIFEST).unwrap();
        assert!(!manifest.metric_sections.assertions.patterns.is_empty());
        assert!(manifest
            .metric_sections
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

    /// Every `@branch.*` capture an adapter's query declares must have a
    /// matching `[branches]` manifest entry (pretender-1v8k.11): without
    /// this guard, an uncaptured entry silently falls back to the 1/1
    /// default and the manifest stops describing the adapter's branch
    /// semantics. Fails loudly per language and capture.
    #[test]
    fn every_branch_capture_has_a_manifest_entry() {
        for record in crate::registry::REGISTRY.iter() {
            let manifest = record.manifest().expect("builtin manifest must parse");
            for capture in branch_capture_names(record.query) {
                assert!(
                    manifest.metric_sections.branches.contains_key(&capture),
                    "{:?}: query capture {capture} has no [branches] manifest entry",
                    record.language
                );
            }
        }
    }

    /// Distinct `@branch.<name>` capture names in query source, in
    /// declaration order (deduplicated).
    fn branch_capture_names(query: &str) -> Vec<String> {
        let mut names = Vec::new();
        let mut rest = query;
        while let Some(pos) = rest.find("@branch.") {
            let tail = &rest[pos + "@branch.".len()..];
            let end = tail
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.'))
                .unwrap_or(tail.len());
            let name = format!("@branch.{}", &tail[..end]);
            if !names.contains(&name) {
                names.push(name);
            }
            rest = &tail[end..];
        }
        names
    }
}
