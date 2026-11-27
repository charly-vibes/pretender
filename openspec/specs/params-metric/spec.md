# params-metric Specification

## Purpose
Defines how the universal code model captures function parameters per code unit and how the `params_max` threshold is enforced against that capture. Rust is specified here because its parameter shape (declared parameters, `self` counting as one) needed an explicit contract when the params metric was wired for it (fix-rust-params-capture); Go/Python behavior is pinned by the same `params` list semantics.

## Requirements

### Requirement: Rust parameter capture

The universal code model SHALL record a non-empty `params` list for Rust functions with declared parameters; `params_max` enforcement SHALL apply to Rust units.

#### Scenario: Rust parameters captured

- WHEN a Rust source file containing a function with two named parameters is parsed
- THEN the resulting unit reports `params` of length 2 (self, when present, counts as one parameter, matching Go/Python behavior)
