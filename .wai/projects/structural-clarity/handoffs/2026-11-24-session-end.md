---
date: 2026-11-24
project: structural-clarity
phase: implementation
---
# Session Handoff

## What Was Done

Foundation (commit e7b2252, one atomic commit — the pre-commit gate scans
the staged tree, so only the fully-fixed tree passes):
- 5zo.18 ticket contract (`docs/our-standard.md`, `.beads/templates/ticket.md`)
- 5zo.21 `just dogfood-red` (un-grandfather exactly path+rule ledger IDs;
  removal-only guard measures the actual mutated output after review)
- 5zo.19 governance gates suite: monotonicity ⊆ origin/main, purity via
  freeze window (`governance/baseline-freeze.json`), threshold floor,
  exclude allowlist, assertion floor (1222)
- 5zo.17 pre-commit gate now runs the checkout build (was the stale global
  0.7.0 scanning gitignored book//target and polluting `.pretender` history)
- 6tt Rust params extraction (tree-sitter `pattern` field + self params;
  openspec proposal `fix-rust-params-capture`) + remediation of the 17
  params_max violations it surfaced (parameter structs, dead params removed)
- gv5 void_mutator canonicalization ((line, name) sort; 3 identical scans)
- ledger prune 164 → 161

Review outcomes:
- FoundationReview: correct, mergeable; 2 hygiene findings fixed (dogfood-red
  guard measures the actual mutation; openspec tracker synced).
- Wave1Review: correct; 2 commit-message arithmetic errors reworded (rebase,
  unpushed branch); coupling analyze doc note restored. (3 function_lines IDs driven below threshold)

Behavior lock (commit d4358d5):
- 5zo.20 golden corpus: `just golden` / `just golden-check` (in ci);
  `governance/golden.json` = canonical analyzer output over the fixture
  corpus; byte-identity = behavior identity (jq filter drops line numbers +
  envelope, sorts everything)

Wave 1 (commits 294cc1f, a6b6744, 4af8086, 2041233; messages corrected by reword after Wave1Review):
- 5zo.23 coupling.rs clean (98-line analyze decomposed: DependencyGraph,
  compute_coupling, threshold_violations, visit_dependency/report_back_edge)
- 5zo.27 duplication.rs clean (detect_clones 3-stage split; leaf_token;
  test unwraps → Result tests with anyhow)
- 5zo.33 mock_detector.rs clean (patterns_for/scan_line/record; order-pinning
  test added pre-extraction)
- 5zo.34 genesis_compile_test.rs clean (6 .expect → unwrap_or_else panic,
  exact std format)
- Each slice: dogfood-red un-grandfather (RED: gate fails with exactly the
  module's findings) → commit (GREEN: hooks all green). Ledger 161 → 149.

## Key Decisions
- Foundation landed atomically: no commit prefix except the fully-fixed tree
  is gate-green (HEAD mutability_metrics.rs carried un-baselined params_max
  findings).
- Purity gate = approved freeze-window design (baseline IDs ⊆ freeze;
  non-production IDs never baselineable).
- Golden canonicalization excludes line numbers so pure refactors keep
  byte-identity; metrics ARE behavior and legitimately require deliberate
  golden regen with reviewed diffs.
- Wave slices are file-disjoint; ledger edits are orchestrator-owned
  (agents never touch pretender-baseline.json).

## Gotchas & Surprises
- lefthook pre-commit scans a snapshot of the tree (effectively staged
  state): committing while subagents hold unstaged work fails the gate on
  their mid-flight files. Sequence commits only when the tree is quiet.
- The globally installed pretender was 0.7.0; both it and the checkout build
  write `.pretender/history` — interleaved event formats corrupted its
  resolution delta ("112 new"). Fixed by routing the hook through the
  checkout build.
- `just dogfood-red` on the default baseline mutates the ledger in place —
  run it only from the orchestrator, one slice at a time.
- File-level (unit-less) violations live in `file_violations` of the JSON
  report, NOT in `units[]` — easy to miss when diffing finding IDs.

## What Took Longer Than Expected
- Hunting the gate's "2 new" vs jq-diff "0 new" discrepancy → file_violations.
- jq precedence: `[(stream) | sort_by(x)]` pipes each element into sort_by
  instead of collecting first — needs `([stream] | sort_by(x))`.
- Genesis header doc-comment: clippy doc-lazy-continuation wants a blank
  `///` line before the Rationale paragraph, not indentation.

## Open Questions
- None blocking. Wave 2 (.24/.25/.26/.28-.32/.35-.40) queued; .2/.3/.4
  (module decompositions) and .16 (CONTRIBUTING standard) follow the waves.

## Next Steps
1. Address Wave1Review findings (agent output), then wave-2 slices
   (model.rs, primitive_obsession.rs, doctor.rs, then the round-trip moves
   .28/.30/.36-.40 — those share a destination pattern, dispatch at most
   one per wave or verify their target files are disjoint).
2. Push authorization: branch `chore/pre-refactor-anchor` carries
   anchor + foundation + golden + wave 1 (PR to main per repo rules).
3. After all waves: 5zo.15 ratchet (tighten thresholds, shrink ledger to
   empty), then the module decompositions (.2/.3/.4).

## Context
### git_status
```
(branch chore/pre-refactor-anchor; tree clean after wave-1 commits)
```
### open_issues
Run `bd ready` — slices .24/.25/.26/.28-.32/.35-.40 are the next queue;
.2/.3/.4/.16 gated on wave completion.
