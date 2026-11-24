# params metric delta (fix-rust-params-capture)

## ADDED Requirements

### Requirement: Rust parameter capture

The universal code model SHALL record a non-empty `params` list for Rust functions with declared parameters; `params_max` enforcement SHALL apply to Rust units.

#### Scenario: Rust parameters captured

- WHEN a Rust source file containing a function with two named parameters is parsed
- THEN the resulting unit reports `params` of length 2 (self, when present, counts as one parameter, matching Go/Python behavior)
