//! Fixture round-trip tests for the engine's type model: each
//! language's real metrics.scm parses its types fixture and the
//! captured TypeDefinitions/CodeUnits are asserted.
//!
//! # Responsibilities
//! Assert TypeDefinition count/kind/span/ordered fields for the Rust
//! and Go fixtures, that generic parameter text never leaks into
//! captured values, and that CodeUnits stay separate from type
//! members.
//!
//! # Rationale
//! Split from the former `#[cfg(test)]` module in `src/engine.rs`
//! (ratcheted in pretender-5zo.15.3) so each test target and each
//! round-trip function stays within the structural caps; assertions
//! are unchanged and `unwrap`/`expect` became sanctioned
//! `unwrap_or_else` panics in the move.

use std::path::Path;

use pretender::engine::QueryEngine;
use pretender::model::{Language, Span, TypeKind};

fn rust_engine() -> QueryEngine {
    QueryEngine::new(
        tree_sitter_rust::LANGUAGE.into(),
        Language::Rust,
        include_str!("../languages/rust/metrics.scm"),
    )
    .unwrap_or_else(|e| panic!("engine: {e:?}"))
}

fn go_engine() -> QueryEngine {
    QueryEngine::new(
        tree_sitter_go::LANGUAGE.into(),
        Language::Go,
        include_str!("../languages/go/metrics.scm"),
    )
    .unwrap_or_else(|e| panic!("engine: {e:?}"))
}

#[test]
fn rust_types_fixture_round_trip() {
    // Reusable harness pattern (tasks 6.1): the language's real
    // metrics.scm parses a fixture file; assert TypeDefinition
    // count/kind/ordered fields. Sibling language batches copy this
    // test + fixture pair verbatim.
    let engine = rust_engine();
    let source = include_str!("fixtures/rust_types_sample.rs");
    let (module, diags) = engine
        .parse(Path::new("rust_types_sample.rs"), source)
        .unwrap_or_else(|e| panic!("parse rust_types_sample.rs: {e:?}"));

    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");

    assert_eq!(
        module.type_definitions.len(),
        2,
        "both structs captured, impl/fn items are not type definitions"
    );

    let person = module
        .type_definitions
        .iter()
        .find(|t| t.name == "Person")
        .unwrap_or_else(|| panic!("Person captured"));
    assert_eq!(person.kind, TypeKind::Struct);
    assert_eq!(person.language, Language::Rust);
    assert_eq!(
        person.span,
        Span {
            start_line: 1,
            end_line: 5
        }
    );
    let person_fields: Vec<(&str, Option<&str>)> = person
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_name.as_deref()))
        .collect();
    assert_eq!(
        person_fields,
        vec![
            ("name", Some("String")),
            ("age", Some("u8")),
            ("email", Some("String")),
        ],
        "named fields only, in declaration order"
    );
}

#[test]
fn rust_types_generics_and_units_round_trip() {
    // Same fixture, generic struct: captured names stay bare
    // identifiers and CodeUnits remain separate from type members.
    let engine = rust_engine();
    let source = include_str!("fixtures/rust_types_sample.rs");
    let (module, _) = engine
        .parse(Path::new("rust_types_sample.rs"), source)
        .unwrap_or_else(|e| panic!("parse rust_types_sample.rs: {e:?}"));

    let pair = module
        .type_definitions
        .iter()
        .find(|t| t.name == "Pair")
        .unwrap_or_else(|| panic!("generic struct captured"));
    assert_eq!(pair.kind, TypeKind::Struct);
    assert!(
        pair.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
        "captured name must be bare identifier, no grammar syntax: {:?}",
        pair.name
    );
    let pair_fields: Vec<(&str, Option<&str>)> = pair
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_name.as_deref()))
        .collect();
    assert_eq!(
        pair_fields,
        vec![("first", Some("T")), ("second", Some("T"))],
        "generic parameter text never leaks into names or type names"
    );

    assert_eq!(module.units.len(), 2);
    let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
    unit_names.sort_unstable();
    assert_eq!(unit_names, vec!["greet", "standalone"]);
    let person = module
        .type_definitions
        .iter()
        .find(|t| t.name == "Person")
        .unwrap_or_else(|| panic!("Person captured"));
    assert!(
        person.fields.iter().all(|f| f.name != "greet"),
        "methods must not be duplicated as fields"
    );
}

#[test]
fn go_types_fixture_round_trip() {
    let engine = go_engine();
    let source = include_str!("fixtures/go_types_sample.go");
    let (module, diags) = engine
        .parse(Path::new("go_types_sample.go"), source)
        .unwrap_or_else(|e| panic!("parse go_types_sample.go: {e:?}"));

    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");

    assert_eq!(
        module.type_definitions.len(),
        2,
        "both struct type specs captured; funcs are not type definitions"
    );

    let person = module
        .type_definitions
        .iter()
        .find(|t| t.name == "Person")
        .unwrap_or_else(|| panic!("Person captured"));
    assert_eq!(person.kind, TypeKind::Struct);
    assert_eq!(person.language, Language::Go);
    assert_eq!(
        person.span,
        Span {
            start_line: 3,
            end_line: 7
        }
    );
    let person_fields: Vec<(&str, Option<&str>)> = person
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_name.as_deref()))
        .collect();
    assert_eq!(
        person_fields,
        vec![
            ("Name", Some("string")),
            ("Age", Some("uint8")),
            ("Email", Some("string")),
        ],
        "named fields only, in declaration order"
    );
}

#[test]
fn go_types_generics_and_units_round_trip() {
    // Same fixture, generic struct: type parameter syntax must not
    // leak into captured values, and CodeUnits stay separate.
    let engine = go_engine();
    let source = include_str!("fixtures/go_types_sample.go");
    let (module, _) = engine
        .parse(Path::new("go_types_sample.go"), source)
        .unwrap_or_else(|e| panic!("parse go_types_sample.go: {e:?}"));

    let bar = module
        .type_definitions
        .iter()
        .find(|t| t.name == "Bar")
        .unwrap_or_else(|| panic!("generic struct captured"));
    assert_eq!(bar.kind, TypeKind::Struct);
    assert!(
        bar.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
        "captured name must be bare identifier, no grammar syntax: {:?}",
        bar.name
    );
    let bar_fields: Vec<(&str, Option<&str>)> = bar
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_name.as_deref()))
        .collect();
    assert_eq!(
        bar_fields,
        vec![("First", Some("T")), ("Second", Some("T"))],
        "type parameter syntax must not leak into captured values"
    );

    assert_eq!(module.units.len(), 2);
    let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
    unit_names.sort_unstable();
    assert_eq!(unit_names, vec!["Greet", "Standalone"]);
    let person = module
        .type_definitions
        .iter()
        .find(|t| t.name == "Person")
        .unwrap_or_else(|| panic!("Person captured"));
    assert!(
        person.fields.iter().all(|f| f.name != "Greet"),
        "methods must not be duplicated as fields"
    );
}
