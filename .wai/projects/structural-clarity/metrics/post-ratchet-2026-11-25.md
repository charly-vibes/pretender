# Post-Ratchet Snapshot — structural-clarity epic (pretender-5zo.15) complete

Captured 2026-11-25, after the ratchet wave (R1–R8 + sweeps S1–S7). Tip `88b4555` on `chore/ratchet`, 35 commits ahead of `origin/main`, NOT pushed.

## Headline
**Ledger: 174 → 0.** `[thresholds]` pinned at epic targets (cyclomatic 5, cognitive 8, function_lines 20, file_lines 300, nesting 2, abc 20, unwrap 0, duplication 3%). `check .` = 0 violations in src+tests. `just ci` green. Gate `0 known (baselined), 0 new`.

| metric | pre-wave4 (post-wave3) | post-ratchet |
|---|---|---|
| ledger rows | 47 | **0** |
| gate (current → now target) caps | file 400/fn 30/abc 30/cog 15/nest 3/unw 5 | **file 300/fn 20/abc 20/cog 8/nest 2/unw 0** |
| violations in src at [thresholds] | 192 (exposure, pre-sweep) | **0** |
| src files / lines | — | 113 / 16,547 |
| test lines | — | 9,903 |
| cargo test passing | ~402 | **430** |
| governance assertion floor | 1222 | 1191 (re-pinned, moved-not-deleted proven) |

## What the wave did
- **R1–R7**: 47 grandfathered rows fixed — test_report, inheritance_depth, engine (1122→deleted→`engine/`), history, config (1007→206), roles, mutation+main CheckArgs.
- **Threshold flip (R8, 67f244b)** exposed 192 more findings tree-wide → **S1–S7** swept them: metrics families, check pipeline, doctor/plugins/events, cli surface, report/init, all 20 test-target rows.
- **Mechanism upgrades**: `dogfood_red_test` decoupled from the committed ledger via inline fixture (the epic's end state — empty ledger — would otherwise break it forever); human.rs file-cap overage caught via sibling RED proof and fixed (human/{mod,palette}).

## Wave pattern notes (for future orchestration)
- File-disjoint containers + shared-ledger prune-by-owner + RED-proof in detached worktree scaled to 7 parallel agents × 2 waves with only transient races (one index-sweep incident, repaired via soft-reset + private-index re-commit).
- Ledger races are real: prunes must re-read the ledger immediately before staging; one sibling's `--adopt` clobber had to be reverted mid-wave.
- A governance floor (`assertions.count`) fights test consolidation — resolution: re-pin with moved-not-deleted proof, documented in `governance/threshold-floor.toml`.

## Follow-ups
- `gate_baseline_ids_are_a_subset_of_origin_main` fails on stale `origin/main` — passes after merge by design.
- `pretender-5zo` epic umbrella remains (was: children .18/.19 governance-gates + docs items; re-check `bd ready`).
- Push/PR for `chore/ratchet` awaits authorization (repo rule: push always asks).
