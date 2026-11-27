---
tags: [pipeline-run:tdd-ro5-2026-11-27-pretender-jg9y-four-heuristic-gaps, pipeline-step:plan]
---

# pretender-jg9y — four heuristic gaps (TDD plan)

## Desired behavior
1. Rust method-call assertion chains count every link: `.assert().success().stdout(..)` == 3 (macros unchanged).
2. `unwraps` never counts `.unwrap_or*` (already true — pin); excludes `.expect(` inside `#[cfg(test)]`-gated modules when file role is test.
3. `mock_count` skips test-support helper paths (any boundary-aligned `support` segment); leaf tests still fire.
4. Full-repo gate reports `exported_params` (library role) identically to scoped runs — role globs also match walker-relative paths.

## Out of scope
New languages, config knobs, sad repo edits, requiring docstrings.

## Test cases (RED first)
- tests/engine_assertions_test.rs: chain counts 3; single `.assert()` counts 1; non-assertion methods count 0.
- tests/check_test/min_assertions_test.rs: 3-link chain fixture staged under tests/ passes min_assertions.
- tests/check_test/unwrap_exclusions_test.rs: unwrap_or-only fixture no unwraps (max 0); cfg(test) 6-expect fixture role=test no unwraps (max default 5); same file role=app still fires; outside-expect control actual==1 with test unwrap_max 0.
- tests/check_test/mock_support_test.rs: tests/support/helper.rs no mock_count; same source at tests/helper_test.rs still fires.
- tests/check_test/exported_scope_test.rs: full-repo JSON reports exported_params for srclib/** glob (parity with scoped run).

## Narrow test command
cargo test -p pretender --test engine_assertions_test --test check_test

## Full verification
cargo test (workspace) + pretender check --mode gate on ../sad (read-only, no --adopt)

## Files likely touched
languages/rust/metrics.scm, src/error_metrics.rs, src/roles.rs, src/role_resolution.rs,
src/check/analyze/{checks.rs,mod.rs}, tests/fixtures/*, tests/check_test/*, tests/engine_assertions_test.rs

