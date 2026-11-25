use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub trait Parser {
    fn parse(&self, path: &Path, source: &str) -> Result<(Module, Vec<Diagnostic>)>;
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

pub trait Metric {
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
    pub type_name: Option<String>,
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
    Julia,
    Ruby,
    C,
    #[serde(rename = "C++")]
    Cpp,
    R,
    #[serde(rename = "C#")]
    CSharp,
    Clojure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Import {
    pub module: String,
    pub name: Option<String>,
    pub alias: Option<String>,
    pub span: Span,
}

/// A struct, class, or record definition captured by a language adapter.
/// Distinct from `CodeUnit`: function-like members of a type remain
/// `CodeUnit` entries and are not duplicated as fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeDefinition {
    pub name: String,
    pub kind: TypeKind,
    pub span: Span,
    pub fields: Vec<Field>,
    pub language: Language,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeKind {
    Struct,
    Class,
    Record,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub type_name: Option<String>,
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
    /// Struct/class/record definitions captured by the language adapter.
    /// Serde-defaulted so modules serialized before type captures existed
    /// still deserialize.
    #[serde(default)]
    pub type_definitions: Vec<TypeDefinition>,
}

impl Default for Module {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            // Language has no neutral variant; constructions that care set
            // it explicitly. Python is the first adapter in the enum.
            language: Language::Python,
            span: Span {
                start_line: 1,
                end_line: 1,
            },
            lines_total: 0,
            lines_code: 0,
            lines_comment: 0,
            units: Vec::new(),
            imports: Vec::new(),
            type_definitions: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodeUnit {
    pub name: String,
    pub kind: UnitKind,
    pub span: Span,
    pub parameters: Vec<Parameter>,
    pub body: Block,
    pub is_exported: bool,
    pub assertions: u32,
    pub parent_class: Option<String>,
    /// Test identity: definition carries a test-registration attribute
    /// (e.g. Rust `#[test]`). Used by the min_assertions rule scope.
    #[serde(default)]
    pub has_test_attribute: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnitKind {
    Function,
    Method,
    Lambda,
    Ctor,
    Initializer,
    /// Test registration form (e.g. Rust `#[test] fn`, Clojure
    /// `deftest`) — asserted by the min_assertions rule scope.
    Test,
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
    pub cyclomatic_weight: u32,
    pub cognitive_weight: u32,
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
    CondClause,
    Recur,
    MatchCase,
    ComprehensionFilter,
}

impl BranchKind {
    /// D4: branch-bearing control forms wrap their body one level
    /// deeper; logical operators are expressions and never nest.
    pub(crate) fn nests_body(self) -> bool {
        !matches!(
            self,
            BranchKind::LogicalAnd | BranchKind::LogicalOr | BranchKind::ComprehensionFilter
        )
    }
}
