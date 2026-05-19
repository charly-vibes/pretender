use anyhow::Result;
use std::path::Path;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};

const JS_QUERY: &str = include_str!("../../languages/javascript/metrics.scm");

pub struct JavaScriptParser;

impl JavaScriptParser {
    fn engine(&self) -> Result<QueryEngine> {
        QueryEngine::new(
            tree_sitter_javascript::LANGUAGE.into(),
            Language::JavaScript,
            JS_QUERY,
        )
    }
}

impl Parser for JavaScriptParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}
