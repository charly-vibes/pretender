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
            type_definitions: Vec::new(),
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

    #[test]
    fn span_lines_counts_inclusive() {
        // Contract: universal-code-model / span-line-count-is-inclusive —
        // a span with start_line = 3 and end_line = 5 covers 3 lines.
        let span = Span {
            start_line: 3,
            end_line: 5,
        };

        assert_eq!(span.lines(), 3);
        assert_eq!(
            Span {
                start_line: 7,
                end_line: 7
            }
            .lines(),
            1
        );
    }

    #[test]
    fn struct_maps_to_type_definition() {
        // Contract: universal-code-model / struct-maps-to-type-definition —
        // a captured struct with two fields becomes one TypeDefinition
        // with kind Struct and two ordered fields.
        let module = Module {
            path: PathBuf::from("src/user.rs"),
            language: Language::Rust,
            span: Span {
                start_line: 1,
                end_line: 4,
            },
            lines_total: 4,
            lines_code: 4,
            lines_comment: 0,
            units: Vec::new(),
            imports: Vec::new(),
            type_definitions: vec![TypeDefinition {
                name: "User".to_string(),
                kind: TypeKind::Struct,
                span: Span {
                    start_line: 1,
                    end_line: 4,
                },
                fields: vec![
                    Field {
                        name: "id".to_string(),
                        type_name: Some("u64".to_string()),
                        span: Span {
                            start_line: 2,
                            end_line: 2,
                        },
                    },
                    Field {
                        name: "email".to_string(),
                        type_name: Some("String".to_string()),
                        span: Span {
                            start_line: 3,
                            end_line: 3,
                        },
                    },
                ],
                language: Language::Rust,
            }],
        };

        assert_eq!(module.type_definitions.len(), 1);
        let def = &module.type_definitions[0];
        assert_eq!(def.name, "User");
        assert_eq!(def.kind, TypeKind::Struct);
        assert_eq!(def.language, Language::Rust);
        let field_names: Vec<&str> = def.fields.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(field_names, vec!["id", "email"]);
        assert_eq!(def.fields[0].type_name.as_deref(), Some("u64"));

        let value = serde_json::to_value(&module).expect("model should serialize to JSON");
        assert_eq!(value["type_definitions"][0]["name"], "User");
        assert_eq!(value["type_definitions"][0]["kind"], "Struct");
        assert_eq!(value["type_definitions"][0]["fields"][1]["type_name"], "String");
        let round_tripped: Module =
            serde_json::from_value(value).expect("model should deserialize from JSON");
        assert_eq!(round_tripped, module);
    }

    #[test]
    fn members_are_not_duplicated() {
        // Contract: universal-code-model / members-are-not-duplicated —
        // a class with fields AND methods holds one TypeDefinition
        // (fields only) plus separate CodeUnit entries for each method.
        let module = Module {
            path: PathBuf::from("src/widget.py"),
            language: Language::Python,
            span: Span {
                start_line: 1,
                end_line: 8,
            },
            lines_total: 8,
            lines_code: 8,
            lines_comment: 0,
            units: vec![CodeUnit {
                name: "resize".to_string(),
                kind: UnitKind::Method,
                span: Span {
                    start_line: 4,
                    end_line: 5,
                },
                parameters: Vec::new(),
                body: Block {
                    span: Span {
                        start_line: 4,
                        end_line: 5,
                    },
                    nesting: 0,
                    children: Vec::new(),
                },
                is_exported: true,
                assertions: 0,
                parent_class: Some("Widget".to_string()),
                has_test_attribute: false,
            }],
            imports: Vec::new(),
            type_definitions: vec![TypeDefinition {
                name: "Widget".to_string(),
                kind: TypeKind::Class,
                span: Span {
                    start_line: 1,
                    end_line: 5,
                },
                fields: vec![Field {
                    name: "count".to_string(),
                    type_name: Some("u32".to_string()),
                    span: Span {
                        start_line: 2,
                        end_line: 2,
                    },
                }],
                language: Language::Python,
            }],
        };

        assert_eq!(module.type_definitions.len(), 1);
        let field_names: Vec<&str> = module.type_definitions[0]
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .collect();
        assert_eq!(field_names, vec!["count"]);
        assert!(!field_names.contains(&"resize"));

        assert_eq!(module.units.len(), 1);
        assert_eq!(module.units[0].name, "resize");
        assert_eq!(module.units[0].kind, UnitKind::Method);
    }

    #[test]
    fn module_type_definitions_default_to_empty() {
        // Contract: universal-code-model / type-captures-are-optional —
        // a legacy serialized Module without a type_definitions key
        // deserializes to an empty list.
        let json = r#"{
            "path": "src/legacy.py",
            "language": "Python",
            "span": { "start_line": 1, "end_line": 1 },
            "lines_total": 1,
            "lines_code": 1,
            "lines_comment": 0,
            "units": [],
            "imports": []
        }"#;

        let module: Module = serde_json::from_str(json).expect("legacy module should deserialize");
        assert!(
            module.type_definitions.is_empty(),
            "type_definitions must default to empty, not error"
        );
    }
}
