---
date: 2026-11-25
project: structural-clarity
phase: complete
---
# Session Handoff

## What Was Done
Ratchet epic pretender-5zo.15 CLOSED: pretender.toml `[thresholds]` pinned at epic targets (cyc 5, cog 8, fn 20, file 300, nest 2, abc 20, unwrap 0, dup 3); ledger 174 → **0**; `check .` = 0 violations in src+tests; `just ci` green; gate `0 known, 0 new`. All on branch `chore/ratchet`, tip `88b4555`, 35 commits ahead of origin/main, NOT pushed.

Execution: 14 file-disjoint containers across two parallel waves — R1–R7 (47 grandfathered rows), R8 flip + S1–S7 sweep (192 target-exposure rows). Every container: verbatim decomposition, tests out of src, zero unwraps, exact RED proof in detached worktrees, hooks-green commits.

## Key Decisions
- Threshold flip committed BEFORE the sweep with a 192-row exposure ledger (gate stays green via baselining; containers prune as they fix) — keeps every mid-wave commit green under the monotonicity model.
- `dogfood_red_test` decoupled from the committed ledger (inline fixture): the epic's end state (empty ledger) would have broken it permanently; filtering is pure string matching so a fixture tests the mechanics.
- Governance `assertions.count` floor re-pinned 1222 → 1191 with moved-not-deleted mapping; restoring duplicate-shape asserts (re-creating lazy_cluster fingerprints) was disapproved.
- Superseded relic children (.2/.3/.4/.5.x) closed with per-file evidence — their goals are met by the wave.

## Gotchas & Surprises
- The 47-row ledger did NOT represent the ratchet's true cost: flipping thresholds exposed 192 more findings in un-ticketed files (~63 files). Budget ratchets for exposure, not just grandfathered rows.
- Shared-ledger races: one plain `git commit` swept siblings' staged rows (repaired via soft-reset + private-index re-commit); one sibling `--adopt` emptied the working ledger mid-wave (reverted). Prunes must re-read the ledger immediately before staging.
- One container grew its own file past the 300 cap while fixing it; caught by another container's RED proof — cross-checks work.

## What Took Longer Than Expected
- Sweep wave (~35 min/container) matched the first wave; the un-ticketed exposure sweep was unplanned scope (+192 rows) but mechanical.

## Open Questions
- None blocking. `gate_baseline_ids_are_a_subset_of_origin_main` fails only on stale origin/main; resolves at merge.

## Next Steps
1. **Push/PR authorization** (awaiting human): branch `chore/ratchet`, 35 commits, PR to main.
2. After merge: verify CI, then `pretender-5zo` umbrella items (governance-gates suite .18/.19, docs) via `bd ready`.
3. Consider ratcheting `excessive_fields_max` (10) and `mut_ratio_max` (0.30) similarly if another tightening pass is wanted.

## Context
### git_status
```
On branch chore/ratchet; clean; tip 88b4555; 35 ahead of origin/main; not pushed
```
### open_issues
Epic 5zo.15 closed (with .2/.3/.4/.5/.5.1-.6, 15.1-15.7, 15.9-15.16). Remaining under 5zo umbrella: 15.18 (ticket contract/template), 15.19 (governance gates suite), plus non-5zo items via `bd ready`.
### metrics
See `../metrics/post-ratchet-2026-11-25.md` (ledger 0, gate 0/0, 430 tests green, floor 1191).
