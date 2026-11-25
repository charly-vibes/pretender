//! Query-engine type-capture tests: the universal code model's
//! TypeDefinition capture rules through the public API.
//!
//! # Responsibilities
//! Assert that type definitions bind fields to the innermost enclosing
//! @type.definition, anonymous inner types stay captured as declared
//! text, uncaptured nested types (C/C++ anonymous unions, Go inline
//! structs — design D3) do not leak members into enclosing structs,
//! and adapters without type captures stay valid.
//!
//! # Rationale
//! Split from the former `#[cfg(test)]` module in `src/engine.rs`
//! (ratcheted in pretender-5zo.15.3) so each test target stays within
//! the file-lines cap; assertions are unchanged and `unwrap`/`expect`
//! became sanctioned `unwrap_or_else` panics in the move.

use std::path::Path;

use pretender::engine::QueryEngine;
use pretender::model::*;

fn go_engine() -> QueryEngine {
    QueryEngine::new(
        tree_sitter_go::LANGUAGE.into(),
        Language::Go,
        include_str!("../languages/go/metrics.scm"),
    )
    .unwrap_or_else(|e| panic!("engine: {e:?}"))
}

fn c_engine() -> QueryEngine {
    QueryEngine::new(
        tree_sitter_c::LANGUAGE.into(),
        Language::C,
        include_str!("../languages/c/metrics.scm"),
    )
    .unwrap_or_else(|e| panic!("engine: {e:?}"))
}

// --- Phase 6: Type captures ---

#[test]
fn type_captures_populate_type_definition() {
    // Contract: universal-code-model / type-captures-populate-type-definition —
    // definition and field captures arrive in separate matches, so fields
    // must bind to the innermost enclosing @type.definition. The tuple
    // field type is an anonymous inner type: captured as declared text,
    // never as its own TypeDefinition. The method stays a CodeUnit.
    let query_source = r#"
(function_item
  name: (identifier) @function.name
  parameters: (parameters) @function.parameters
  body: (block) @function.body) @function.definition

(struct_item
  name: (type_identifier) @type.name) @type.definition

(field_declaration
  name: (field_identifier) @type.field.name
  type: [(type_identifier) (tuple_type)] @type.field.type) @type.field
"#;
    let engine = QueryEngine::new(
        tree_sitter_rust::LANGUAGE.into(),
        Language::Rust,
        query_source,
    )
    .unwrap_or_else(|e| panic!("engine: {e:?}"));
    let source = "struct Outer {\n    name: String,\n    inner: (i32, bool),\n}\n\nimpl Outer {\n    fn method(&self) -> u32 {\n        self.name.len() as u32\n    }\n}\n";
    let (module, diags) = engine
        .parse(Path::new("test.rs"), source)
        .unwrap_or_else(|e| panic!("parse test.rs: {e:?}"));

    assert!(diags.is_empty(), "type captures must not emit diagnostics");

    assert_eq!(
        module.type_definitions.len(),
        1,
        "exactly one TypeDefinition"
    );
    let def = &module.type_definitions[0];
    assert_eq!(def.name, "Outer");
    assert_eq!(def.kind, TypeKind::Struct);
    assert_eq!(def.language, Language::Rust);
    assert_eq!(
        def.span,
        Span {
            start_line: 1,
            end_line: 4
        }
    );

    let field_names: Vec<&str> = def.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        field_names,
        vec!["name", "inner"],
        "only struct-level fields, in declaration order"
    );
    assert_eq!(def.fields[0].type_name.as_deref(), Some("String"));
    assert_eq!(
        def.fields[1].type_name.as_deref(),
        Some("(i32, bool)"),
        "anonymous inner type captured as declared text"
    );
    assert!(
        def.fields.iter().all(|f| f.name != "method"),
        "methods must not be duplicated as fields"
    );

    assert_eq!(module.units.len(), 1);
    assert_eq!(module.units[0].name, "method");
    assert_eq!(module.units[0].kind, UnitKind::Function);
}

#[test]
fn c_anonymous_type_members_not_attributed_to_enclosing_struct() {
    // Contract: universal-code-model — members of deliberately
    // uncaptured nested types (C/C++ anonymous unions/structs, design
    // D3) belong to the inner type, not to the enclosing captured
    // struct. File-global field queries must not leak raw/scaled/calib
    // into Sensor's field list.
    let engine = c_engine();
    let source = "\
struct Sensor {
    int reading;
    union {
        int raw;
        double scaled;
    };
    struct {
        int calib;
    };
    int unit;
};
";
    let (module, diags) = engine
        .parse(Path::new("sensor.c"), source)
        .unwrap_or_else(|e| panic!("parse sensor.c: {e:?}"));

    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
    assert_eq!(
        module.type_definitions.len(),
        1,
        "anonymous union/struct stay uncaptured (design D3)"
    );
    let sensor = &module.type_definitions[0];
    let field_names: Vec<&str> = sensor.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        field_names,
        vec!["reading", "unit"],
        "anonymous union/struct members are not Sensor fields"
    );
}

#[test]
fn go_inline_struct_members_not_attributed_to_enclosing_struct() {
    // Contract: universal-code-model — an inline struct type
    // (uncaptured per design D3) nested inside a captured struct's
    // field must not leak its members into the enclosing definition,
    // and an inline struct inside a function stays outside the type
    // model entirely.
    let engine = go_engine();
    let source = "\
package main

type Config struct {
    Options struct {
        Verbose bool
    }
    Name string
}

func run() {
    var local struct {
        Depth int
    }
    _ = local
}
";
    let (module, diags) = engine
        .parse(Path::new("config.go"), source)
        .unwrap_or_else(|e| panic!("parse config.go: {e:?}"));

    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
    assert_eq!(
        module.type_definitions.len(),
        1,
        "inline struct types are not captured (design D3)"
    );
    let config = &module.type_definitions[0];
    let field_names: Vec<&str> = config.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        field_names,
        vec!["Options", "Name"],
        "inline struct members are not Config fields"
    );
}

#[test]
fn adapter_without_type_captures_stays_valid() {
    // Contract: universal-code-model / adapter-without-type-captures-stays-valid —
    // a query with only @function.* captures parses to an empty
    // type_definitions list with no error diagnostic.
    let query_source = r#"
(function_definition
  name: (identifier) @function.name
  parameters: (parameters) @function.parameters
  body: (block) @function.body) @function.definition
"#;
    let engine = QueryEngine::new(
        tree_sitter_python::LANGUAGE.into(),
        Language::Python,
        query_source,
    )
    .unwrap_or_else(|e| panic!("engine: {e:?}"));
    let source = "def hello(x):\n    return x\n";
    let (module, diags) = engine
        .parse(Path::new("test.py"), source)
        .unwrap_or_else(|e| panic!("parse test.py: {e:?}"));

    assert!(
        diags.is_empty(),
        "missing type captures must not be a parse failure"
    );
    assert!(module.type_definitions.is_empty());
    assert_eq!(module.units.len(), 1);
    assert_eq!(module.units[0].name, "hello");
}
