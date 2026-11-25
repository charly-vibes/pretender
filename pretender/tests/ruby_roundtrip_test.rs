//! Ruby parser integration tests.
//!
//! # Responsibilities
//! Pin Ruby-specific parse behavior through the public parser API.
//!
//! # Rationale
//! The round-trip unit test moved out of the `#[cfg(test)]` module in
//! `src/ruby.rs` (pretender-948q registry migration): it pins the Ruby
//! adapter's zero-capture contract, and this is its matching integration
//! target.

use pretender::model::Parser;
use pretender::registry::{lookup, GenericParser};

/// The production record for rb, through the public registry API.
fn parser_for_rb() -> GenericParser {
    match lookup("rb") {
        Some(record) => GenericParser { record },
        None => panic!("rb must be registered"),
    }
}
use std::path::Path;

#[test]
fn ruby_types_zero_capture_round_trip() {
    // Design D2: Ruby ships with ZERO type captures — attr_accessor /
    // attr_writer are method calls in the grammar, not field nodes.
    // The documented zero-capture case: the adapter parses cleanly
    // with an empty type_definitions list.
    let parser = parser_for_rb();
    let source = include_str!("fixtures/ruby_sample.rb");
    let (module, diags) = parser.parse(Path::new("ruby_sample.rb"), source).unwrap();

    assert!(diags.is_empty(), "fixture must parse cleanly: {diags:?}");
    assert!(
        module.type_definitions.is_empty(),
        "Ruby adapter must produce no type definitions"
    );
    assert!(
        !module.units.is_empty(),
        "ruby methods must still be captured as CodeUnits"
    );
}
