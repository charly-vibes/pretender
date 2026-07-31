# Change: Add mutable-state check

## Why

Excessive mutable state (`let`/`var` over `const`/`val`/`final`, void-return mutation methods, defensive-copy absence) is a leading indicator of temporal coupling, side-effect entanglement, and poor test isolation. A deterministic ratio check lets teams gate against creeping mutability before it becomes architectural debt.

## What Changes

- Add a per-file mutable-state ratio metric that counts mutable binding sites (`let`, `var`, non-final fields) against total binding sites, computed from AST captures.
- Introduce `mut_ratio_max: f64` under `[thresholds]` (default `0.0` = disabled), acting as a multiplier threshold: if `mut_ratio > mut_ratio_max` the file is flagged.
- Per-language binding patterns: Rust (`let mut`, `mut` params), JS/TS (`let`, `var` vs `const`), Java (`non-final` local var), C++ (`mutable`, `auto`, non-const).
- **Python excluded**: Python has no `const` keyword, and `global` is a scope modifier, not a mutability signal. The check does not apply to Python files.
- Excludes files with role `generated` or `vendor`.
- Flagged files get a `MutationRatioFinding` on `FileReport`; gatable in `gate` mode.
- Purely static — no execution needed.

## Impact

- Affected specs: `cli-and-config` (ADDED mutable-state metric threshold and report field).
- Affected code: `pretender/src/config.rs` (new `mut_ratio_max` field), `pretender/src/main.rs` (evaluation path, new finding type, exit-code extension), new `pretender/src/mutability_metrics.rs` (ratio computation per language).
- No change to the universal code model — computed from existing AST captures or per-language queries.
- Non-breaking: default `0.0` disables the check.

## Dependencies

- **Must be implemented together with `add-void-mutator-check`** — both share `mutability_metrics.rs`. Should be sequenced after `add-coupling-and-cohesion-metrics` if the shared pattern registry is in place, but can be implemented independently with a local `mutability_metrics.rs` that is later merged.