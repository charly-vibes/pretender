// Purpose: Round-trip integration test for the C# parser against its real
//          metrics.scm query and the csharp_types_sample.cs fixture.
// Responsibilities: Assert TypeDefinition count/kind/span/ordered fields for
//                   Person and Box, record kind mapping, and that CodeUnits
//                   stay separate from type members.
// Rationale: Moved verbatim from pretender/src/csharp.rs unit tests (slice
//            5zo.37) so the source file passes pretender's own structural
//            gate; assertions are unchanged. The fixture round trip is
//            split into focused tests (person fields, generic fields, units
//            separation) sharing the parse helper; every assertion is
//            unchanged.

use std::path::Path;

use pretender::csharp::CSharpParser;
use pretender::model::{Language, Module, Parser, Span, TypeDefinition, TypeKind};

fn engine() -> CSharpParser {
    CSharpParser
}

/// Reusable harness pattern (tasks 6.3): the language's real metrics.scm
/// parses the committed fixture; asserts a clean parse and returns the module.
fn parse_csharp_types_fixture() -> Module {
    let source = include_str!("fixtures/csharp_types_sample.cs");
    let (module, diags) = engine()
        .parse(Path::new("csharp_types_sample.cs"), source)
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"));

    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
    module
}

/// Finds the type definition named `name` or panics naming it.
fn find_type<'a>(module: &'a Module, name: &str) -> &'a TypeDefinition {
    module
        .type_definitions
        .iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("{name} captured"))
}

#[test]
fn csharp_person_class_fields_round_trip() {
    let module = parse_csharp_types_fixture();

    assert_eq!(
        module.type_definitions.len(),
        2,
        "both classes captured; methods are not type definitions"
    );

    let person = find_type(&module, "Person");
    assert_eq!(person.kind, TypeKind::Class);
    assert_eq!(person.language, Language::CSharp);
    assert_eq!(
        person.span,
        Span {
            start_line: 3,
            end_line: 13
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
            ("Age", Some("int")),
            ("Email", Some("string")),
        ],
        "named fields only, in declaration order"
    );
}

#[test]
fn csharp_box_generic_fields_round_trip() {
    let module = parse_csharp_types_fixture();

    let boxed = find_type(&module, "Box");
    assert_eq!(boxed.kind, TypeKind::Class);
    assert!(
        boxed.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
        "captured name must be bare identifier, no grammar syntax: {:?}",
        boxed.name
    );
    let box_fields: Vec<(&str, Option<&str>)> = boxed
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_name.as_deref()))
        .collect();
    assert_eq!(
        box_fields,
        vec![("Value", Some("T"))],
        "type parameter syntax must not leak into captured values"
    );
}

#[test]
fn csharp_units_separate_from_type_members() {
    let module = parse_csharp_types_fixture();
    let person = find_type(&module, "Person");

    // CodeUnits remain separate from type members.
    assert_eq!(module.units.len(), 2);
    let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
    unit_names.sort_unstable();
    assert_eq!(unit_names, vec!["Get", "Greet"]);
    assert!(
        person.fields.iter().all(|f| f.name != "Greet"),
        "methods must not be duplicated as fields"
    );
}

#[test]
fn csharp_record_maps_to_record_kind() {
    // The pinned tree-sitter-c-sharp grammar (0.23.5) exposes
    // `record_declaration` as the only node for record declarations
    // (there is no record_struct_declaration); record components arrive
    // as a parameter_list of parameter nodes. The node kind contains
    // "record", so determine_type_kind maps it to TypeKind::Record.
    let source = "public record Point(int X, string Y) {\n    public int Get() {\n        return X;\n    }\n}\n";
    let (module, diags) = engine()
        .parse(Path::new("Point.cs"), source)
        .unwrap_or_else(|e| panic!("record must parse: {e:?}"));

    assert!(diags.is_empty(), "record must parse cleanly: {diags:?}");
    assert_eq!(module.type_definitions.len(), 1);
    let point = &module.type_definitions[0];
    assert_eq!(point.name, "Point");
    assert_eq!(point.kind, TypeKind::Record);
    let fields: Vec<(&str, Option<&str>)> = point
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_name.as_deref()))
        .collect();
    assert_eq!(
        fields,
        vec![("X", Some("int")), ("Y", Some("string"))],
        "record components captured as ordered fields"
    );
    assert_eq!(module.units.len(), 1);
    assert_eq!(module.units[0].name, "Get");
}
