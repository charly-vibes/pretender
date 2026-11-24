---
tags: [pipeline-run:tdd-ro5u-2026-11-24-dogfood-pretender-gate-in-ci, pipeline-step:red]
---

# RED: dogfood gate wiring is missing

Added `pretender/tests/repo_dogfood_test.rs` — repo-level governance tests that
assert the structural gate is wired into `just ci` and therefore into CI.

First run: 4 failed, 2 passed.
- `justfile_defines_a_dogfood_recipe` — justfile had no `dogfood` recipe.
- `just_ci_includes_the_dogfood_gate` — `ci: fmt-check type-check lint test` (no gate).
- `pretender_config_is_in_gate_mode` — pretender.toml said `mode = "tiered"` with no
  `advisory_until` lease, so a bare `pretender check . --staged` (the command
  AGENTS.md mandates) errored out instead of checking.
- `pretender_config_excludes_test_fixtures` — fixtures (deliberate counterexamples)
  were inside the gate scope, dominating the finding list with 90+ intentional hits.

Passing already: CI workflow runs `just ci`; baseline file is committed.

## GREEN
- justfile: new `dogfood` recipe (`cargo run --quiet -p pretender -- check .`);
  `ci:` now ends with `dogfood`.
- pretender.toml: `mode = "gate"`, `exclude` gains `**/fixtures/**`.
- .github/workflows/ci.yml: comment documenting that `just ci` now gates.
- Baseline re-adopted (164 findings; fixture entries dropped).

## Verification
- `cargo test --test repo_dogfood_test` → 2 passed.
- `just ci` → fmt-check, type-check, clippy, tests, gate all pass; gate reports
  `164 known (baselined), 0 new`.
- Three consecutive `check .` scans → identical `164 known / 0 new` (no flapping,
  which was the risk from pretender-gv5).
- `ah check` → clean.

## Note discovered while implementing
The first version of the test file used six near-identical test bodies; pretender's
own `lazy_cluster` rule flagged it (`lazy_cluster 3 > 3`) — the tool caught the new
code immediately. Rewritten as a table-driven test with two distinct bodies.

