---
date: 2026-11-24
project: structural-clarity
phase: wave-3 complete, unpushed
---
# Session Handoff
## What Was Done
Wave 3 of pretender-5zo (epic: make pretender's own code easy to understand)
landed entirely on branch `chore/wave3` — 33 commits ahead of origin/main,
NOT pushed (push needs explicit authorization per repo constraints).

- Container 5zo.2: main.rs 3977 → 237 lines via report/, check/, init/,
  events.rs, cli/ extractions (all RED-proven with detached-worktree gates).
- Container 5zo.3: engine.rs 1749 → 1122 via parse.rs, mutation_entry.rs,
  metric_application.rs, type_definitions.rs, coupling.rs, role_resolution.rs.
- Container 5zo.4: cli_test.rs (3900 lines) deleted; 124 tests conserved
  across check_test/init_test/tooling_test/report_format_test.
- Singles: .29 mutability_metrics clean, .16 CONTRIBUTING standard,
  .22 reviewer-bead wiring (both force-closed past the .15 sequencing note).
- Repo-wide rustfmt canonicalization + semantic extractions keeping the
  gate ratchet intact (commit `9043496`); roles re-export bridge dropped
  (`27d7bf6`). `just ci` fully green; ledger 124 → 47 rows.
## Key Decisions
- Ratchet discipline: ledger only shrinks; every prune RED-proven by running
  the gate in a detached worktree at the pre-commit SHA with the post-prune
  ledger (IDs must re-fire exactly).
- `main.rs::::excessive_fields` (CheckArgs, 13 fields) deliberately kept —
  substruct wiring would change construction sites for no structural gain.
- rustfmt default widths reject formatting-level compactions; only semantic
  extractions survive — recorded in `9043496`'s message.
## Gotchas & Surprises
- `dogfood_red_test.rs` pinned `main.rs::run::function_lines_max` as fixture;
  `.2.5` removed those rows by design → selector now derives from the ledger.
- Shared `pretender-baseline.json` across concurrent agents caused a
  sweep/race at `.2.4`/`.4.4`; resolved by tip-consistency check.
- bd children chained per container (blocked_by); close in order or --force
  with a reason.
## What Took Longer Than Expected
- The fmt sweep: seven borderline sites crossed thresholds from reformatting
  alone; each needed a real extraction (gate finding(), severity_marker(),
  line_counts.rs, collect.rs join-collapse, SARIF result-shape helper).
## Open Questions
- Push/PR for `chore/wave3` awaits authorization (33 commits).
- `uwz` corpus calibration run is HITL.
## Next Steps
1. Get authorization → push `chore/wave3` → PR → review → merge to main.
2. Resume `.15` ratchet (thresholds up, ledger 47 → 0) — next epic phase.
3. `.19` governance gates suite; `.17` pre-commit global-install fix;
   `.18` ticket contract; `uwz` HITL corpus run.
4. Remaining grandfathered big files (unticketed): engine.rs 1122,
   config.rs 1007, test_report.rs 870, history.rs 800, mutation.rs 626.
## Context
### git_status
```
(branch chore/wave3, tip 27d7bf6, clean tree; 33 commits ahead of origin/main)
```
### metrics
See `.wai/projects/structural-clarity/metrics/post-wave3-2026-11-24.md`.
