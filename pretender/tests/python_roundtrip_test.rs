// Purpose: Round-trip integration test for the Python parser against its real
//          metrics.scm query and the python_types_sample.py fixture.
// Responsibilities: Assert TypeDefinition count/kind/span/ordered fields for
//                   Person and Account, decorator-free dataclass capture, and
//                   that CodeUnits stay separate from type members.
// Rationale: Moved verbatim from pretender/src/python.rs unit tests (slice
//            5zo.40) so the source file passes pretender's own structural
//            gate; assertions are unchanged. The fixture round trip is
//            split into focused tests (person fields, account fields, units
//            separation) sharing the parse helper; every assertion is
//            unchanged.

use std::path::Path;

use pretender::model::{Language, Module, Parser, Span, TypeDefinition, TypeKind, UnitKind};
use pretender::python::PythonParser;

/// Reusable harness pattern (tasks 6.2): the language's real metrics.scm
/// parses the committed fixture; asserts a clean parse and returns the module.
fn parse_python_types_fixture() -> Module {
    let source = include_str!("fixtures/python_types_sample.py");
    let (module, diags) = PythonParser
        .parse(Path::new("python_types_sample.py"), source)
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
fn python_types_fixture_round_trip() {
    let module = parse_python_types_fixture();

    assert_eq!(
        module.type_definitions.len(),
        2,
        "both classes captured; methods are not type definitions"
    );

    let person = find_type(&module, "Person");
    // Design D3: @dataclass ships as Class — no decorator refinement.
    assert_eq!(person.kind, TypeKind::Class);
    assert_eq!(person.language, Language::Python);
    assert_eq!(
        person.span,
        Span {
            start_line: 7,
            end_line: 16
        }
    );
    assert!(
        person.name.chars().all(|c| c.is_alphanumeric() || c == '_'),
        "captured name must be bare identifier, no grammar syntax: {:?}",
        person.name
    );
    let person_fields: Vec<(&str, Option<&str>)> = person
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_name.as_deref()))
        .collect();
    assert_eq!(
        person_fields,
        vec![
            ("name", Some("str")),
            ("age", Some("int")),
            ("email", Some("str")),
        ],
        "annotated attributes only, in declaration order"
    );
    assert!(
        person.fields.iter().all(|f| f.name != "greeting"),
        "method-local annotated variables must not become fields"
    );
}

#[test]
fn python_account_class_round_trip() {
    let module = parse_python_types_fixture();

    // Second half of the fixture round trip: the Account class has its own
    // span, and __init__ self.x assignments are not duplicated as fields.
    let account = find_type(&module, "Account");
    assert_eq!(account.kind, TypeKind::Class);
    assert_eq!(
        account.span,
        Span {
            start_line: 19,
            end_line: 27
        }
    );
    let account_fields: Vec<(&str, Option<&str>)> = account
        .fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_name.as_deref()))
        .collect();
    assert_eq!(
        account_fields,
        vec![("balance", Some("float")), ("owner", Some("str"))],
        "__init__ self.x assignments must not be duplicated as fields"
    );
}

#[test]
fn python_units_separate_from_type_members() {
    let module = parse_python_types_fixture();

    // CodeUnits remain separate from type members.
    let mut unit_names: Vec<&str> = module.units.iter().map(|u| u.name.as_str()).collect();
    unit_names.sort_unstable();
    assert_eq!(unit_names, vec!["__init__", "greet"]);
    let greet = module
        .units
        .iter()
        .find(|u| u.name == "greet")
        .unwrap_or_else(|| panic!("greet captured"));
    assert_eq!(greet.kind, UnitKind::Method);
    let ctor = module
        .units
        .iter()
        .find(|u| u.name == "__init__")
        .unwrap_or_else(|| panic!("__init__ captured"));
    assert_eq!(ctor.kind, UnitKind::Ctor);
}
