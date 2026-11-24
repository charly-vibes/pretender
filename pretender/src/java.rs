// Purpose: Java language parser plugin for the pretender analyzer.
// Responsibilities: Build the Java QueryEngine from the language's real
//                   metrics.scm query and plugin.toml manifest (cached), and
//                   expose JavaParser implementing the Parser trait.
// Rationale: Unit tests moved verbatim to pretender/tests/java_roundtrip_test.rs
//            (slice 5zo.38) so this source file passes pretender's own
//            structural gate; behavior is unchanged.

use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const JAVA_QUERY: &str = include_str!("../languages/java/metrics.scm");
const JAVA_MANIFEST: &str = include_str!("../languages/java/plugin.toml");

pub struct JavaParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(JAVA_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl JavaParser {
    pub fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_java::LANGUAGE.into(),
            Language::Java,
            JAVA_QUERY,
            &crate::engine::WeightMaps {
                branch: &m.branches,
                call: &m.smell_weights,
            },
        )
    }
}

impl Parser for JavaParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}
