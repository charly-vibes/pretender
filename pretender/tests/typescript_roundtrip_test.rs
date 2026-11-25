// Purpose: Round-trip integration test for the TypeScript parser against its
//          real metrics.scm query and the ts_types_sample.ts fixture.
// Responsibilities: Assert TypeDefinition count/kind/span/ordered fields for
//                   Person and Pair, bare-identifier name capture, and that
//                   CodeUnits stay separate from type members; plus the
//                   members-not-duplicated contract for function-valued class
//                   properties.
// Rationale: Moved verbatim from pretender/src/typescript.rs unit tests
//            (slice 5zo.30) so the source file passes pretender's own
//            structural gate; assertions are unchanged, unwraps converted to
//            the unwrap_or_else/panic! pattern with std-identical reporting.
//            The fixture round trip is split into focused tests (person
//            fields, generic fields, units separation) sharing the parse
//            helper; every assertion is unchanged.

use std::path::Path;

use pretender::model::{Language, Module, Parser, Span, TypeDefinition, TypeKind, UnitKind};
use pretender::registry::{lookup, GenericParser};

/// The production record for ts, through the public registry API.
fn parser_for_ts() -> GenericParser {
    match lookup("ts") {
        Some(record) => GenericParser { record },
        None => panic!("ts must be registered"),
    }
}

/// Reusable harness pattern (tasks 6.4): the language's real metrics.scm
/// parses the committed fixture; asserts a clean parse and returns the module.
fn parse_ts_types_fixture() -> Module {
    let source = include_str!("fixtures/ts_types_sample.ts");
    let (module, diags) = parser_for_ts()
        .parse(Path::new("ts_types_sample.ts"), source)
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
fn ts_person_class_fields_round_trip() {
    let module = parse_ts_types_fixture();

    assert_eq!(
        module.type_definitions.len(),
        2,
        "both classes captured; standalone fn is not a type definition"
    );

    let person = find_type(&module, "Person");
    assert_eq!(person.kind, TypeKind::Class);
    assert_eq!(person.language, Language::TypeScript);
    assert_eq!(
        person.span,
        Span {
            start_line: 1,
            end_line: 9
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
            ("name", Some("string")),
            ("age", Some("number")),
            ("email", Some("string")),
        ],
        "typed class fields only, in declaration order, type text from the annotation"
    );
}

#[test]
fn ts_pair_generic_fields_round_trip() {
    let module = parse_ts_types_fixture();

    let pair = find_type(&module, "Pair");
    assert_eq!(pair.kind, TypeKind::Class);
    assert!(
        pair.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
        "captured name must be bare identifier, no type-parameter syntax: {:?}",
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
        "type parameter text must not leak into captured values"
    );
}

#[test]
fn ts_units_separate_from_type_members() {
    let module = parse_ts_types_fixture();
    let person = find_type(&module, "Person");

    // CodeUnits remain separate from type members.
    assert_eq!(module.units.len(), 3);
    let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
    unit_names.sort_unstable();
    assert_eq!(unit_names, vec!["greet", "standalone", "swap"]);
    assert!(
        person.fields.iter().all(|f| f.name != "greet"),
        "methods must not be duplicated as fields"
    );
}

#[test]
fn ts_function_valued_class_properties_are_units_not_fields() {
    // Contract: members-not-duplicated — function-valued class
    // properties are CodeUnits (kind follows the existing TS
    // convention, where method_definition members are Function too)
    // and never data fields.
    let parser = parser_for_ts();
    let source = "\
class Widget {
  label: string = 'ok';

  onClick = (): void => {
    return;
  };

  render(): string {
    return this.label;
  }
}
";
    let (module, diags) = parser
        .parse(Path::new("widget.ts"), source)
        .unwrap_or_else(|e| panic!("fixture must parse: {e:?}"));

    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");

    let widget = module
        .type_definitions
        .iter()
        .find(|t| t.name == "Widget")
        .unwrap_or_else(|| panic!("Widget captured"));
    let field_names: Vec<&str> = widget.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(
        field_names,
        vec!["label"],
        "arrow-valued property is a CodeUnit, not a data field"
    );

    let unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(
        unit_names,
        vec!["onClick", "render"],
        "function-valued property surfaces as a CodeUnit like methods"
    );
    for name in ["onClick", "render"] {
        let unit = module
            .units
            .iter()
            .find(|u| u.name == name)
            .unwrap_or_else(|| panic!("unit {name} captured"));
        assert_eq!(
            unit.kind,
            UnitKind::Function,
            "{name} follows the existing TS member kind convention"
        );
    }
    assert!(
        widget.fields.iter().all(|f| f.name != "onClick"),
        "members must not be duplicated as fields"
    );
}
