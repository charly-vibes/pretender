//! C parser integration tests.
//!
//! # Responsibilities
//! Pin C-specific parse behavior through the public parser API.
//!
//! # Rationale
//! The round-trip unit tests moved out of the `#[cfg(test)]` module in
//! `src/c.rs` (pretender-948q registry migration): they pin the C parser's
//! capture behavior, not adapter wiring, and this is their matching
//! integration target.

use pretender::registry::{lookup, GenericParser};

/// The production record for c, through the public registry API.
fn parser_for_c() -> GenericParser {
    match lookup("c") {
        Some(record) => GenericParser { record },
        None => panic!("c must be registered"),
    }
}
use pretender::model::{Module, Parser, TypeDefinition, TypeKind};
use std::path::Path;

const C_TYPES_SAMPLE: &str = include_str!("fixtures/c_types_sample.c");

/// The captured struct is Person with three fields of fixed names and
/// types; the free function stays a CodeUnit, not duplicated as a field.
fn assert_person_shape(module: &Module, def: &TypeDefinition) {
    assert_eq!(def.name, "Person");
    assert_eq!(def.kind, TypeKind::Struct);
    assert_eq!(def.language, pretender::model::Language::C);

    let names: Vec<&str> = def.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["name", "age", "height"]);
    let types: Vec<Option<&str>> = def.fields.iter().map(|f| f.type_name.as_deref()).collect();
    assert_eq!(types, [Some("char"), Some("int"), Some("double")]);

    assert!(module.units.iter().any(|u| u.name == "person_age"));
    assert!(def.fields.iter().all(|f| f.name != "person_age"));
}

#[test]
fn c_types_sample_round_trip() {
    let parser = parser_for_c();
    let (module, diagnostics) = parser
        .parse(Path::new("tests/fixtures/c_types_sample.c"), C_TYPES_SAMPLE)
        .unwrap_or_else(|e| panic!("c_types_sample.c must parse: {e:?}"));
    assert!(diagnostics.is_empty(), "unexpected: {diagnostics:?}");

    // Structs are captured; unions are not (design D3).
    assert_eq!(
        module.type_definitions.len(),
        1,
        "the union must not yield a TypeDefinition"
    );
    assert_person_shape(&module, &module.type_definitions[0]);
}
