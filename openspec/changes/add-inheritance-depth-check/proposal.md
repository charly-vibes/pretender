# Change: Add inheritance-depth check

## Why

Deep inheritance hierarchies (class A extends B extends C extends D) are a well-known structural rigidity signal — base-class changes cascade through subclasses, and deep chains resist refactoring. A deterministic depth gate lets teams flag files exceeding a configurable inheritance depth before the hierarchy becomes fragile.

## What Changes

- Add a per-file (or per-class) inheritance depth computation: count the transitive `extends` chain from each class.
- Introduce `inheritance_depth_max: u32` under `[thresholds]` (default `0` = disabled).
- Detection: walk the AST for `extends` / `:` / `impl` / `<:` / `inherits` declarations in each class and count chain length.
- Findings attach per `CodeUnit` in `UnitReport.violations` with metric name `inheritance_depth` when depth > threshold.
- Per-language: Java (`extends`), Python (`class A(B)`), JS/TS (`extends`), C++ (`:`), Ruby (`<`), C# (`:`).
- **Rust trait bounds excluded**: `trait A: B + C` where B or C is a standard library trait (Display, Debug, Clone, etc.) is NOT counted as inheritance. Only custom trait inheritance is counted, and even then it counts as depth 1 at most (trait inheritance is not transitive in the same way as class inheritance).
- **Single-file only**: Does NOT require cross-file resolution — flag based on declared parent depth only. If a parent class is in another file, depth = 1 (declared parent) with a note that full depth is unknown. Cross-file chains are not resolved.
- Excludes files with role `generated` or `vendor`.

## Impact

- Affected specs: `cli-and-config` (ADDED inheritance-depth threshold field, violation metric name).
- Affected code: `pretender/src/config.rs` (new `inheritance_depth_max`), `pretender/src/main.rs` (evaluation in per-unit metric pass, violation creation), per-language adapters (add `@superclass` / `@parent` capture to class queries if not already present).
- Minimal — uses existing `CodeUnit` tree extended with a parent-class field.
- Non-breaking: default `0` disables the check.

## Dependencies

- None (independent).