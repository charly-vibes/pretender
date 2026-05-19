use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const TS_QUERY: &str = include_str!("../../languages/typescript/metrics.scm");
const TS_MANIFEST: &str = include_str!("../../languages/typescript/plugin.toml");

pub struct TypeScriptParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(TS_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl TypeScriptParser {
    fn engine(&self) -> Result<QueryEngine> {
        QueryEngine::new_with_branch_weights(
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::TypeScript,
            TS_QUERY,
            &manifest()?.branches,
        )
    }
}

impl Parser for TypeScriptParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}

pub struct TypeScriptXParser;

impl TypeScriptXParser {
    fn engine(&self) -> Result<QueryEngine> {
        QueryEngine::new_with_branch_weights(
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            Language::TypeScript,
            TS_QUERY,
            &manifest()?.branches,
        )
    }
}

impl Parser for TypeScriptXParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}
