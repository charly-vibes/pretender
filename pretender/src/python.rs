use anyhow::Result;
use std::path::Path;

use crate::engine::QueryEngine;
use crate::model::{Diagnostic, Language, Module, Parser};

pub struct PythonParser;

impl PythonParser {
    fn engine(&self) -> Result<QueryEngine> {
        QueryEngine::new(
            tree_sitter_python::LANGUAGE.into(),
            Language::Python,
            include_str!("../../queries/python.scm"),
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
