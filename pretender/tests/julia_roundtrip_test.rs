//! Purpose: Integration test for the Julia parser's round-trip behavior on the
//! julia_types_sample fixture.
//! Responsibilities: Parse the fixture via the generic parser, assert captured type
//! definitions, fields, kinds, language tagging, and method-as-unit handling.
//! Rationale: Lives in tests/ to keep pretender/src/julia.rs free of inline test
//! modules (function_lines gate) while asserting identical behavior.

use std::path::Path;

use pretender::registry::{lookup, GenericParser};

/// The production record for jl, through the public registry API.
fn parser_for_jl() -> GenericParser {
    match lookup("jl") {
        Some(record) => GenericParser { record },
        None => panic!("jl must be registered"),
    }
}
use pretender::model::{Language, Parser, Span, TypeKind};

const JULIA_TYPES_SAMPLE: &str = include_str!("fixtures/julia_types_sample.jl");

#[test]
fn julia_types_fixture_round_trip() {
    // Reusable harness pattern (tasks 6.1/6.6): the language's real
    // metrics.scm parses a fixture file; assert TypeDefinition
    // count/kind/ordered fields and that CodeUnits stay separate.
    // struct and mutable struct both parse as struct_definition and
    // must both land as Struct with their fields in declaration order.
    let parser = parser_for_jl();
    let source = JULIA_TYPES_SAMPLE;
    let (module, diags) = parser
        .parse(Path::new("julia_types_sample.jl"), source)
        .unwrap();

    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");

    assert_eq!(
        module.type_definitions.len(),
        2,
        "struct and mutable struct captured, functions are not type definitions"
    );

    let person = module
        .type_definitions
        .iter()
        .find(|t| t.name == "Person")
        .expect("Person captured");
    assert_eq!(person.kind, TypeKind::Struct);
    assert_eq!(person.language, Language::Julia);
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
            ("age", Some("Int")),
            ("email", Some("String")),
        ],
        "typed field declarations in source order"
    );

    let pair = module
        .type_definitions
        .iter()
        .find(|t| t.name == "Pair")
        .expect("parametric mutable struct captured");
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
        "type parameter syntax must not leak into captured values"
    );
}

#[test]
fn julia_type_members_stay_code_units() {
    // CodeUnits remain separate from type members: methods of the structs
    // and standalone functions are captured as units, not duplicated as
    // fields of their enclosing type definitions.
    let parser = parser_for_jl();
    let source = JULIA_TYPES_SAMPLE;
    let (module, diags) = parser
        .parse(Path::new("julia_types_sample.jl"), source)
        .unwrap();
    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");

    let person = module
        .type_definitions
        .iter()
        .find(|t| t.name == "Person")
        .expect("Person captured");

    assert_eq!(module.units.len(), 2);
    let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
    unit_names.sort_unstable();
    assert_eq!(unit_names, vec!["greet", "standalone"]);
    assert!(
        person.fields.iter().all(|f| f.name != "greet"),
        "methods must not be duplicated as fields"
    );
}
