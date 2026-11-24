# Change: Fix Rust params capture (params metric for Rust extractor)

## Why

Dogfooding shows `params=0` for every Rust unit, which means `AppThresholds.params_max` (default 4) is silently unenforced for Rust. The tree-sitter query already captures `@function.parameters` for Rust functions, but the shared `extract_params` helper in the engine only reads a `name` field on parameter children. tree-sitter-rust `parameter` nodes have a `pattern` field (no `name` field), and `self_parameter`/`self` children aren't handled at all — so every Rust parameter resolves to no name and is dropped, yielding an empty params list for every Rust unit.

## What Changes

- Wire (repair) the existing `params` metric for the Rust adapter: extend the shared engine `extract_params` helper to read Rust's `pattern` field (falling back to `name`, preserving Python/Go behavior) and to recognize `self_parameter` / `self` nodes as a `self` parameter, mirroring how Go/Python already populate params.
- NO threshold value changes: `params_max` stays 4 everywhere. No rule outcome rules change; this is a capture-fidelity fix that makes an existing rule actually apply to Rust.
- No other language adapter touched; universal model unchanged (params metric and `Parameter` shape already exist).

## Impact

- Affected specs: none added/changed — this repairs conformance with the existing params metric behavior spec (ADDED scenario only if the repo's spec requires an explicit Rust example).
- Affected code: `pretender/src/engine.rs` (`extract_params` field fallback + `self_parameter` handling), `pretender/src/config.rs` (additive test only).
- Non-breaking: only ever adds parameters that were previously dropped; findings may newly appear where `params_max` was not enforced for Rust (measured and reported, baseline not edited in this change).
