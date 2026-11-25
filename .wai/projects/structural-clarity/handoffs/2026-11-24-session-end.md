---
date: 2026-11-24
project: structural-clarity
phase: implementation
---

# Session Handoff

## Status (end of wave 2, branch chore/wave2)

- PR #62 (foundation + golden + wave 1) **merged to main** (f2aa13f).
- Wave 2 implemented on chore/wave2 (base 9514c9e → tip f08ff96):
  - Prerequisite: **lib target split** — src/lib.rs declares all 35 modules
    pub; main.rs imports via `use pretender::…`. Integration tests can now
    import the crate (the round-trip-move tickets were impossible without
    this). Config::parse_str stayed lib-side; two main.rs test callers
    inline toml::from_str.
  - 13 slices landed with per-slice RED→GREEN (stash dance):
    .24 model.rs, .25 primitive_obsession.rs, .26 doctor.rs, .28
    javascript.rs, .30 typescript.rs, .31 data_model.rs, .32
    external_plugin.rs, .35 cluster_detector.rs, .36 cpp.rs, .37 csharp.rs,
    .38 java.rs, .39 julia.rs, .40 python.rs — all gate-clean; round-trip
    tests now live in tests/*_roundtrip_test.rs (+ model_test,
    primitive_obsession_test).
  - Ledger 149 → **124** (25 IDs pruned, every removal earned).
  - `just ci` exit 0; golden-check byte-identical; clippy -D warnings clean.
  - Gate fix-ups during wave 2: needless borrow in doctor.rs; test-role
    params_max=2 forced a base_unit builder; lazy_cluster (test threshold 3,
    structural fingerprints) required structural differentiation of test
    helpers — detector groups units by (calls, callees, asserts, branches),
    not text similarity.
- Reviews: FoundationReview + Wave1Review (wave 1, both "correct", findings
  addressed); Wave2Review dispatched, pending at handoff time.
- Tickets closed: .18 .19(.1–.5) .20 .21 .17 .23 .27 .33 .34 .24 .25 .26
  .28 .30 .31 .32 .35 .36–.40, pretender-6tt, pretender-gv5.

## Next Steps (wave 3 queue)

1. Address Wave2Review findings, merge the wave-2 PR.
2. pretender-5zo.29 mutability_metrics.rs (3 violations) — last full-clean
   slice.
3. .2 main.rs decomposition, .3 engine.rs, .4 cli_test.rs split (round-trip
   moves freed the language src files; these remain the big rocks).
4. .16 CONTRIBUTING structural standard; 5zo.15 ratchet (tighten thresholds,
   shrink ledger toward empty); .22 per-ticket independence wiring.

## Gotchas

- The pre-commit gate scans the WORKTREE (staged tree): never commit while
  slice agents hold unrelated files mid-edit (wave-1 lesson, still true).
- `just dogfood-red <path> <rule>` is idempotent-safe to re-run; prune
  before the RED demo or the ledger will still absorb the findings.
- Test-role thresholds are TIGHTER than app for params_max (2) and have
  lazy_cluster_min = 3: moved tests must be structurally differentiated.
- GitHub CI checkout needs fetch-depth: 0 for the monotonicity gate
  (fixed in ci.yml during wave 1's PR).
