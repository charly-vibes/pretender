#![allow(dead_code)]

use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub start_line: u32,
    pub end_line: u32,
}

impl Span {
    pub fn lines(&self) -> u32 {
        self.end_line - self.start_line + 1
    }
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct CallSite {
    pub callee: String,
    pub span: Span,
    pub smell_weight: f64,
}

#[derive(Debug, Clone)]
pub enum Language {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Go,
    Java,
    Ruby,
    C,
    Cpp,
    CSharp,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub path: PathBuf,
    pub language: Language,
    pub span: Span,
    pub lines_total: u32,
    pub lines_code: u32,
    pub lines_comment: u32,
    pub units: Vec<CodeUnit>,
}

#[derive(Debug, Clone)]
pub struct CodeUnit {
    pub name: String,
    pub kind: UnitKind,
    pub span: Span,
    pub parameters: Vec<Parameter>,
    pub body: Block,
    pub is_exported: bool,
}

#[derive(Debug, Clone)]
pub enum UnitKind {
    Function,
    Method,
    Lambda,
    Ctor,
    Initializer,
}

#[derive(Debug, Clone)]
pub struct Block {
    pub span: Span,
    pub nesting: u32,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone)]
pub enum Node {
    Statement(Span),
    Branch(Branch),
    NestedBlock(Block),
    Call(CallSite),
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub kind: BranchKind,
    pub span: Span,
    pub nesting_at: u32,
}

#[derive(Debug, Clone)]
pub enum BranchKind {
    If,
    ElseIf,
    SwitchCase,
    Loop,
    Catch,
    Ternary,
    Logical,
    NullCoalesce,
    EarlyReturn,
}
