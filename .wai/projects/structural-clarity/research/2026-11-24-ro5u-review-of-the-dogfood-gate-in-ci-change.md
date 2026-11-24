---
tags: [pipeline-run:tdd-ro5u-2026-11-24-dogfood-pretender-gate-in-ci, pipeline-step:review]
---

# RO5U: review of the dogfood-gate-in-CI change

## Stage 1 — DRAFT
Shape: GOOD. One recipe (`just dogfood`), one config change (`mode = "gate"` +
fixture exclusion), one governance test, one workflow comment. Small, no product
code touched, no spec touched.

## Stage 2 — CORRECTNESS
- CORR-001 (HIGH, fixed): the first draft of the governance test asserted the exact
  `ci:` line `ci: fmt-check type-check lint test dogfood`. Brittle: reordering or
  adding a dependency to `ci` breaks the test for no good reason. Now the test
  finds the `ci:` line and asserts it contains `dogfood`.
- CORR-002 (HIGH, found by the tool itself): the first test file had six
  near-identical test bodies; pretender's own `lazy_cluster` rule flagged the new
  file (`lazy_cluster 3 > 3`). Rewritten as a table-driven test with distinct bodies.
- CORR-003 (HIGH, confirmed empirically, NOT fixed here — epic input): baseline
  finding IDs are `file::symbol::rule` for function rules and `file::::rule` for
  file rules. Verified in a scratch project: renaming `fn alpha` → `fn beta` turned
  2 baselined findings into "2 new" and failed the gate. Consequence: **every
  refactor that renames or splits a function must ship a baseline refresh
  (`pretender check . --adopt`) in the same PR**, and the reviewer must read the
  baseline diff to confirm it only reflects renames/moves. Without that, the epic's
  refactor PRs will fail the gate for cosmetic reasons.
- Verified: `cargo run --quiet -p pretender -- check .` works from the workspace
  root; `just ci` passes end to end; three consecutive scans report an identical
  `164 known / 0 new` (no flapping, the pretender-gv5 risk); `ah check` is clean.

## Stage 3 — CLARITY
Comments in `justfile`, `pretender.toml`, and `ci.yml` explain *why* (gate +
baseline = only new findings fail; fixtures are deliberate counterexamples).

## Stage 4 — EDGE CASES
- EDGE-001 (MEDIUM, documented, not fixed): a PR that modifies the tool can make
  the gate pass trivially — the gate runs the code in the checkout by design
  (that is what dogfooding means). Accepted risk.
- EDGE-002 (MEDIUM, documented): `pretender-baseline.json` missing → gate mode is
  fail-closed on every finding → CI fails loudly. Good default; the governance
  test asserts the file is committed.
- EDGE-003 (LOW, out of scope): `.github/workflows/ci.yml` uploads
  `target/release/pretender` but no step builds it in release mode — the artifact
  step is currently ineffective. Pre-existing, unrelated to the gate; left alone
  to keep this change minimal.

## Stage 5 — EXCELLENCE
- Convergence: after Stage 2 fixes, 0 new CRITICAL, no new findings.
- Verdict: READY, with CORR-003 carried into the epic as an explicit working
  agreement ("refactor PRs ship a baseline refresh; reviewers read the diff").

