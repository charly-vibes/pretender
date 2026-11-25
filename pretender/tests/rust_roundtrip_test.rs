//! Rust parser integration tests.
//!
//! # Responsibilities
//! Pin Rust-specific parse behavior through the public parser API.
//!
//! # Rationale
//! `rust_sample_function_yields_nonzero_params` moved out of the
//! `#[cfg(test)]` module in `src/config.rs` (wave 4 ratchet): it pins the
//! Rust parser's params capture, not config behavior, and this is its
//! matching integration target.

use pretender::model::Parser;
use pretender::rust::RustParser;
use std::path::Path;

#[test]
fn rust_sample_function_yields_nonzero_params() {
    // pretender-6tt: params metric must apply to Rust so params_max is
    // enforced; grammar trap: avoid locals named `raw`, tree-sitter-rust
    // 0.23.3 mis-parses them.
    let source = "fn sample(alpha: u32, beta: u32) -> u32 {\n    alpha + beta\n}\n";
    let (module, _diagnostics) = RustParser
        .parse(Path::new("sample.rs"), source)
        .unwrap_or_else(|e| panic!("parse rust sample: {e:?}"));
    assert_eq!(module.units.len(), 1, "sample has one function");
    let unit = &module.units[0];
    assert_eq!(unit.name, "sample");
    assert!(
        !unit.parameters.is_empty(),
        "params metric must be populated for Rust functions"
    );
    assert_eq!(unit.parameters.len(), 2);
    assert_eq!(unit.parameters[0].name, "alpha");
    assert_eq!(unit.parameters[1].name, "beta");
}
