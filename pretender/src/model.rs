#![allow(dead_code)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub trait Parser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)>;
    fn extensions(&self) -> &[&str];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub message: String,
    pub span: Option<Span>,
    pub severity: DiagnosticSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Warning,
    Error,
}

pub struct ParserRegistry {
    parsers: Vec<Box<dyn Parser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        Self {
            parsers: Vec::new(),
        }
    }

    pub fn register(&mut self, parser: Box<dyn Parser>) {
        self.parsers.push(parser);
    }

    pub fn get_for_extension(&self, ext: &str) -> Option<&dyn Parser> {
        self.parsers
            .iter()
            .find(|p| p.extensions().contains(&ext))
            .map(|p| p.as_ref())
    }
}

pub trait Metric {
    fn name(&self) -> &'static str;
    fn calculate(&self, unit: &CodeUnit) -> u32;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start_line: u32,
    pub end_line: u32,
}

impl Span {
    pub fn lines(&self) -> u32 {
        assert!(
            self.end_line >= self.start_line,
            "Span end_line must be >= start_line"
        );
        self.end_line - self.start_line + 1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operand {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operator {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallSite {
    pub callee: String,
    pub span: Span,
    pub smell_weight: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Go,
    Java,
    Ruby,
    C,
    #[serde(rename = "C++")]
    Cpp,
    #[serde(rename = "C#")]
    CSharp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Import {
    pub module: String,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub path: PathBuf,
    pub language: Language,
    pub span: Span,
    pub lines_total: u32,
    pub lines_code: u32,
    pub lines_comment: u32,
    pub units: Vec<CodeUnit>,
    pub imports: Vec<Import>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeUnit {
    pub name: String,
    pub kind: UnitKind,
    pub span: Span,
    pub parameters: Vec<Parameter>,
    pub body: Block,
    pub is_exported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitKind {
    Function,
    Method,
    Lambda,
    Ctor,
    Initializer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub span: Span,
    pub nesting: u32,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Node {
    Statement(Span),
    Branch(Branch),
    NestedBlock(Block),
    Call(CallSite),
    Assignment(Span),
    Operand(Operand),
    Operator(Operator),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Branch {
    pub kind: BranchKind,
    pub span: Span,
    pub nesting_at: u32,
    pub sequence_id: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BranchKind {
    If,
    ElseIf,
    SwitchCase,
    Loop,
    Catch,
    Ternary,
    LogicalAnd,
    LogicalOr,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn module_serializes_to_json_with_imports() {
        let module = Module {
            path: PathBuf::from("src/example.py"),
            language: Language::Python,
            span: Span {
                start_line: 1,
                end_line: 3,
            },
            lines_total: 3,
            lines_code: 2,
            lines_comment: 1,
            units: Vec::new(),
            imports: vec![Import {
                module: "collections".to_string(),
                name: Some("Counter".to_string()),
                alias: None,
                span: Span {
                    start_line: 1,
                    end_line: 1,
                },
            }],
        };

        let value = serde_json::to_value(&module).expect("model should serialize to JSON");
        let round_tripped: Module =
            serde_json::from_value(value.clone()).expect("model should deserialize from JSON");

        assert_eq!(value["language"], "Python");
        assert_eq!(value["imports"][0]["module"], "collections");
        assert_eq!(value["imports"][0]["name"], "Counter");
        assert_eq!(round_tripped, module);
    }

    #[test]
    fn branch_kind_distinguishes_logical_operator_direction() {
        let left = BranchKind::LogicalAnd;
        let right = BranchKind::LogicalOr;

        assert_ne!(left, right);
    }
}
