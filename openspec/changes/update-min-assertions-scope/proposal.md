# Proposal: update-min-assertions-scope

## Why

After pretender-5bw fixed assertion-macro parsing, every remaining `min_assertions`
finding on dogfood scans is a *real* zero-assertion unit that is not a test:
112 units in `tests/fixtures/` sample files (e.g. `simple`, `with_branch`) and 22
helper functions in test files (`tempdir`, `pretender_bin`, `check`). They are
flagged only because role detection assigns the file role `test` from the
`tests/` directory path, and `min_assertions` currently applies to every
test-role unit.

These flags are pure noise on the highest-volume rule: they teach agents and
humans to distrust the advisory channel (measured: 0 in-session edits after
violations across 5 repos), which blocks the gate-enforcement work in
pretender-15w.

## What Changes

- `min_assertions` SHALL be evaluated only for code units with **test identity**:
  1. the unit carries a test-registration attribute in the source language
     (e.g. Rust `#[test]`), or
  2. the unit name matches a built-in test-name pattern
     (`^test_`, `_test$`, `^test[A-Z]`, `Test$`, `^test$`).
- Units in test-role files without test identity (fixtures, helpers) are exempt
  from `min_assertions`; all other rules (complexity, cognitive, params, …)
  keep their current behavior on those units.
- No thresholds change. No config schema change (built-in detection only).
- Role detection (`cli-and-config` → Role Detection) is unchanged; scoping
  happens at rule evaluation, not role assignment.

## Impact

- Affected specs: new capability `min-assertions-rule` (ADDED requirements).
- Affected code:
  - `pretender/src/engine.rs` (rule evaluation: test-identity check)
  - `pretender/src/roles.rs` (no change; reference only)
  - `pretender/tests/cli_test.rs` (red→green tests)
- Expected outcome on dogfood self-scan: remaining 134 `min_assertions` events
  drop to ~0 (22 helper + 112 fixture units all lack test identity).
- Advisory/gate exit-code behavior for genuine tests unchanged (advisory stays
  default; gate mode is pretender-15w's scope).
