use anyhow::Result;
use std::path::Path;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};

const TS_QUERY: &str = include_str!("../../languages/typescript/metrics.scm");

pub struct TypeScriptParser;

impl TypeScriptParser {
    fn engine(&self) -> Result<QueryEngine> {
        QueryEngine::new(
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Language::TypeScript,
            TS_QUERY,
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
        QueryEngine::new(
            tree_sitter_typescript::LANGUAGE_TSX.into(),
            Language::TypeScript,
            TS_QUERY,
        )
    }
}

impl Parser for TypeScriptXParser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)> {
        self.engine()?.parse(path, source)
    }
}
