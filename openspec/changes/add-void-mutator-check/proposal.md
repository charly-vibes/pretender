# Change: Add void-mutator check

## Why

Methods that return `void`/`None`/`nil` while mutating `self`/`this` are a sign of imperative style that makes testing harder — callers can't track state changes, and the mutation pattern resists functional-core extraction. A deterministic count per file helps teams recognise when a file is accumulating too many void mutators and should be refactored toward return-new-instance patterns.

## What Changes

- Add a per-file count of void-return methods that mutate instance state (access `self`/`this` fields with assignment operators).
- Introduce `void_mutators_max: u32` under `[thresholds]` (default `0` = disabled).
- Detect via: method return type is void/unit + body contains assignment to `self.`/`this.` fields via direct assignment operator (`self.field = value`, `this.field = value`). Method calls on child objects (`self.list.add(x)`) are NOT counted as direct mutation — only direct field assignment is flagged.
- Void-mutator methods are flagged in `UnitReport.violations` with metric name `void_mutator` when their count per file exceeds the threshold.
- Per-language: Rust (`fn ...(&mut self)` with no return), Python (`def` that mutates `self` with no return), Java (`void` method with `this.x =`), JS/TS (method mutating `this` with no `return`), etc.
- Only applies to non-test, non-generated, non-vendor roles by default (test helpers often legitimately mutate state; configurable via `[roles.test] void_mutators_max` if desired).

## Impact

- Affected specs: `cli-and-config` (ADDED void-mutator threshold, violation metric name, role-specific override).
- Affected code: `pretender/src/config.rs` (new `void_mutators_max` on thresholds + role overrides), `pretender/src/main.rs` (per-unit analysis pass, violation creation), new `pretender/src/mutability_metrics.rs` (shared with mutable-state proposal; adds void-mutator detection).
- Requires per-language AST captures for `self`/`this` mutation (assignment to field expressions).
- Non-breaking: default `0` disables the check.

## Dependencies

- **Must be implemented together with `add-mutable-state-check`** — both share `mutability_metrics.rs`.