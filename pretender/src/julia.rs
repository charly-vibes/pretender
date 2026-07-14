use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const JULIA_QUERY: &str = include_str!("../languages/julia/metrics.scm");
const JULIA_MANIFEST: &str = include_str!("../languages/julia/plugin.toml");

pub struct JuliaParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(JULIA_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl JuliaParser {
    fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_julia::LANGUAGE.into(),
            Language::Julia,
            JULIA_QUERY,
            &m.branches,
            &m.smell_weights,
        )
    }
}

impl Parser for JuliaParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}
