use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::OnceLock;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};
use crate::plugin::PluginManifest;

const CSHARP_QUERY: &str = include_str!("../languages/csharp/metrics.scm");
const CSHARP_MANIFEST: &str = include_str!("../languages/csharp/plugin.toml");

pub struct CSharpParser;

fn manifest() -> Result<&'static PluginManifest> {
    static MANIFEST: OnceLock<std::result::Result<PluginManifest, String>> = OnceLock::new();
    match MANIFEST
        .get_or_init(|| PluginManifest::from_toml(CSHARP_MANIFEST).map_err(|err| err.to_string()))
    {
        Ok(manifest) => Ok(manifest),
        Err(err) => Err(anyhow!(err.clone())),
    }
}

impl CSharpParser {
    fn engine(&self) -> Result<QueryEngine> {
        let m = manifest()?;
        QueryEngine::new_with_branch_weights(
            tree_sitter_c_sharp::LANGUAGE.into(),
            Language::CSharp,
            CSHARP_QUERY,
            &m.branches,
            &m.smell_weights,
        )
    }
}

impl Parser for CSharpParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}
