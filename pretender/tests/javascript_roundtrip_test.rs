// Purpose: Round-trip integration test for the JavaScript language query
//          against its real metrics.scm and the js_types_sample.js fixture.
// Responsibilities: Assert TypeDefinition count/kind/span/ordered fields for
//                   Person and Pair, that CodeUnits stay separate from type
//                   members, and that function-valued class properties are
//                   CodeUnits rather than data fields.
// Rationale: Moved verbatim from pretender/src/javascript.rs unit tests (slice
//            5zo.28) so the source file passes pretender's own structural
//            gate; assertions are unchanged. The fixture round trip is
//            split into focused tests (person fields, pair fields, units
//            separation) sharing the parse helper; every assertion is
//            unchanged.

use std::path::Path;

use pretender::engine::QueryEngine;
use pretender::model::{Language, Module, Span, TypeDefinition, TypeKind, UnitKind};

const JS_QUERY: &str = include_str!("../languages/javascript/metrics.scm");

fn engine() -> QueryEngine {
    QueryEngine::new(
        tree_sitter_javascript::LANGUAGE.into(),
        Language::JavaScript,
        JS_QUERY,
    )
    .unwrap_or_else(|e| panic!("QueryEngine::new should succeed: {e:?}"))
}

/// Reusable harness pattern (tasks 6.4): the language's real metrics.scm
/// parses the committed fixture; asserts a clean parse and returns the module.
fn parse_js_types_fixture() -> Module {
    let source = include_str!("fixtures/js_types_sample.js");
    let (module, diags) = engine()
        .parse(Path::new("js_types_sample.js"), source)
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
fn js_person_class_fields_round_trip() {
    let module = parse_js_types_fixture();

    assert_eq!(
        module.type_definitions.len(),
        2,
        "both classes captured; standalone fn is not a type definition"
    );

    let person = find_type(&module, "Person");
    assert_eq!(person.kind, TypeKind::Class);
    assert_eq!(person.language, Language::JavaScript);
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
        vec![("name", None), ("age", None), ("email", None)],
        "class fields only, in declaration order; JS has no type annotations"
    );
}

#[test]
fn js_pair_generic_fields_round_trip() {
    let module = parse_js_types_fixture();

    let pair = find_type(&module, "Pair");
    assert_eq!(pair.kind, TypeKind::Class);
    assert!(
        pair.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
        "captured name must be bare identifier, no grammar syntax: {:?}",
        pair.name
    );
    let pair_fields: Vec<&str> = pair.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(pair_fields, vec!["first", "second"]);
}

#[test]
fn js_units_separate_from_type_members() {
    let module = parse_js_types_fixture();
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
fn js_function_valued_class_properties_are_units_not_fields() {
    // Contract: members-not-duplicated — function-valued class
    // properties are CodeUnits (kind follows the existing JS
    // convention, where method_definition members are Function too)
    // and never data fields.
    let e = engine();
    let source = "\
class Widget {
  label = 'ok';
  count = 0;

  onClick = () => {
    this.count += 1;
  };

  render() {
    return this.label;
  }
}
";
    let (module, diags) = e
        .parse(Path::new("widget.js"), source)
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
        vec!["label", "count"],
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
            .unwrap_or_else(|| panic!("{name} captured"));
        assert_eq!(
            unit.kind,
            UnitKind::Function,
            "{name} follows the existing JS member kind convention"
        );
    }
    assert!(
        widget.fields.iter().all(|f| f.name != "onClick"),
        "members must not be duplicated as fields"
    );
}
