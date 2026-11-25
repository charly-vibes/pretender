// Purpose: Integration tests for the universal code model (pretender::model).
// Responsibilities: Verify serialization/round-tripping, span line counting,
//   type-definition capture semantics, and member (field vs. method) separation.
// Rationale: Extracted verbatim from src/model.rs's inline test module to keep
//   the source file within gate thresholds; assertions and test names unchanged.
use pretender::model::{
    Block, BranchKind, CodeUnit, Field, Import, Language, Module, Span, TypeDefinition, TypeKind,
    UnitKind,
};
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

    let value = serde_json::to_value(&module)
        .unwrap_or_else(|e| panic!("model should serialize to JSON: {e:?}"));
    let round_tripped: Module = serde_json::from_value(value.clone())
        .unwrap_or_else(|e| panic!("model should deserialize from JSON: {e:?}"));

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

    let value = serde_json::to_value(&module)
        .unwrap_or_else(|e| panic!("model should serialize to JSON: {e:?}"));
    assert_eq!(value["type_definitions"][0]["name"], "User");
    assert_eq!(value["type_definitions"][0]["kind"], "Struct");
    assert_eq!(
        value["type_definitions"][0]["fields"][1]["type_name"],
        "String"
    );
    let round_tripped: Module = serde_json::from_value(value)
        .unwrap_or_else(|e| panic!("model should deserialize from JSON: {e:?}"));
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

    let module: Module = serde_json::from_str(json)
        .unwrap_or_else(|e| panic!("legacy module should deserialize: {e:?}"));
    assert!(
        module.type_definitions.is_empty(),
        "type_definitions must default to empty, not error"
    );
}
