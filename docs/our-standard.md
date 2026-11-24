# Our Standard: the per-ticket structural-clarity contract

Every ticket in the structural-clarity epic follows the same 8 steps. Each
step produces a named artifact and has a machine-checked proof — "done" is
never a matter of opinion.

## The 8 steps

| # | Step | Artifact | Machine-checked proof |
|---|------|----------|----------------------|
| 1 | **Research** — why this seam, with evidence | Research note in the ticket (links, measurements, call-site evidence) | Reviewer rejects tickets whose research cites no concrete evidence; the design step cannot start without it |
| 2 | **Design** — module responsibility map + extraction plan | Responsibility map and extraction plan committed with the ticket | The plan names the exact module boundaries being moved; no plan, no RED |
| 3 | **RED** — un-grandfather the finding IDs | `just dogfood-red <path> <rule>` removes the ticket's IDs from `pretender-baseline.json`; the gate fails on the un-refactored code | `just dogfood-red <path> <rule>` exits non-zero: the structural gate reports the findings the baseline no longer covers |
| 4 | **GREEN** — refactor, no behavior change | Refactor commit(s); the gate passes again | `just dogfood` (and `just ci`) exits zero with the shrunken baseline |
| 5 | **TIDY** | A separate commit touching only naming, dead code, comments, and leftover shims | Diff of the TIDY commit contains no behavior change; gates stay green |
| 6 | **Independent review** | A different agent runs `/ro5` (or a parallel-review wave); the verdict is committed as a wai review artifact | The review artifact exists in the repo; unaddressed findings block the ticket |
| 7 | **Mechanical gates green** | — | `just ci` green: fmt, type-check, lint, tests, structural gate |
| 8 | **Ledger update** — the removed IDs stay removed | Ledger delta recorded in the ticket (IDs removed, none re-added) | The baseline-monotonicity gate: `pretender-baseline.json` may only shrink; the delta in the ticket matches the diff |

## The ledger model

`pretender-baseline.json` is not a suppression list — it is the **burn-down
list** of grandfathered findings.

- **The baseline IS the ledger.** Every entry is a known finding we have
  committed to fixing before the epic closes.
- **Additions are forbidden.** New findings must be fixed, not
  grandfathered. This is enforced mechanically by the baseline-monotonicity
  gate: any diff that grows the baseline fails.
- **The baseline may only shrink inside the epic.** Tickets remove IDs at
  RED (step 3) and keep them removed through GREEN (step 4) and the ledger
  update (step 8).
- **Only ticket 5zo.15 (the ratchet) may re-adopt IDs**, and only after
  thresholds tighten — i.e. re-adoption happens exclusively as part of a
  deliberate, reviewed tightening, never as a convenience during a
  refactor ticket.
