use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const JS_QUERY: &str = include_str!("../languages/javascript/metrics.scm");
const JS_MANIFEST: &str = include_str!("../languages/javascript/plugin.toml");

pub struct JavaScriptParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(JS_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl JavaScriptParser {
    fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_javascript::LANGUAGE.into(),
            Language::JavaScript,
            JS_QUERY,
            &crate::engine::WeightMaps {
                branch: &m.branches,
                call: &m.smell_weights,
            },
        )
    }
}

impl Parser for JavaScriptParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}

// Purpose: JavaScript language parser plugin for the pretender analyzer.
// Responsibilities: Build a QueryEngine from the real javascript metrics.scm
//                   query and plugin.toml manifest; expose JavaScriptParser
//                   implementing the Parser trait (parse → Module +
//                   Diagnostics).
// Rationale: Round-trip assertions live in tests/javascript_roundtrip_test.rs
//            so this source file stays clean under pretender's structural gate
//            (moved verbatim by slice 5zo.28).
