//! Purpose: Integration test for the C++ parser's round-trip behavior on the
//! cpp_types_sample fixture.
//! Responsibilities: Parse the fixture via `CppParser`, assert captured type
//! definitions, fields, kinds, language tagging, and method-as-unit handling.
//! Rationale: Lives in tests/ to keep pretender/src/cpp.rs free of inline test
//! modules (function_lines gate) while asserting identical behavior.

use std::path::Path;

use pretender::cpp::CppParser;
use pretender::model::{Language, Parser, TypeKind};

const CPP_TYPES_SAMPLE: &str = include_str!("fixtures/cpp_types_sample.cpp");

#[test]
fn cpp_types_sample_round_trip() {
    let parser = CppParser;
    let (module, diagnostics) = parser
        .parse(
            Path::new("tests/fixtures/cpp_types_sample.cpp"),
            CPP_TYPES_SAMPLE,
        )
        .unwrap();
    assert!(diagnostics.is_empty(), "unexpected: {diagnostics:?}");

    assert_eq!(module.type_definitions.len(), 2);
    let widget = module
        .type_definitions
        .iter()
        .find(|d| d.name == "Widget")
        .expect("class captured");
    // class_specifier contains neither "record" nor "struct" → Class.
    assert_eq!(widget.kind, TypeKind::Class);
    assert_eq!(widget.language, Language::Cpp);

    let names: Vec<&str> = widget.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(names, ["width", "title", "visible"]);
    let types: Vec<Option<&str>> = widget
        .fields
        .iter()
        .map(|f| f.type_name.as_deref())
        .collect();
    assert_eq!(types, [Some("int"), Some("char"), Some("bool")]);

    let point = module
        .type_definitions
        .iter()
        .find(|d| d.name == "Point")
        .expect("struct captured");
    assert_eq!(point.kind, TypeKind::Struct);
    let point_names: Vec<&str> = point.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(point_names, ["x", "y"]);

    // The method stays a CodeUnit and is not duplicated as a field.
    assert!(module.units.iter().any(|u| u.name == "area"));
    assert!(widget.fields.iter().all(|f| f.name != "area"));
}
