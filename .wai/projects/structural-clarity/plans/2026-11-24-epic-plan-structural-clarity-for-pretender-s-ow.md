---
tags: [pipeline-run:tdd-ro5u-2026-11-24-dogfood-pretender-gate-in-ci, pipeline-step:verify-close]
---

# Epic plan: structural clarity for pretender's own code

Beads epic: **pretender-5zo** (16 child tickets + 2 reparented bugs).

## Why
The tool measures structural simplicity in other people's code but ships its own
3977-line `main.rs`, a 3900-line test file, and 174 grandfathered gate
violations. The credibility of the product rests on its own code being an
example of what it recommends.

## Measurement (baseline, 2026-11-24, `pretender check . --format json`)
174 violations in `pretender/src` + `pretender/tests`:

| rule | violations | limit |
|------|------------|-------|
| function_lines | 90 | 30 |
| nesting_max | 17 | 3 |
| unwraps (per file) | 14 | 5 |
| abc | 13 | 30 |
| cognitive | 12 | 15 |
| file_lines | 10 | 400 |
| cyclomatic | 8 | 10 |
| excessive_fields | 6 | 10 |
| mock_count | 2 | 5 |
| lazy_cluster | 2 | 3 |

Hotspots: `main.rs` (3977 lines / 54 violations), `engine.rs` (1749 / 26),
`cli_test.rs` (3900 / 276 mocks / 294 unwraps), `test_report.rs` (869 / 12),
`config.rs` (982), `mutation.rs` (990), `history.rs` (812), `roles.rs` (563).

## Target end state
cyclomatic ≤ 5, cognitive ≤ 8, function_lines ≤ 20, file_lines ≤ 300,
nesting ≤ 2, abc ≤ 20, unwraps 0 in app code, duplication ≤ 3% — enforced by the
CI gate with a baseline containing no production-code findings.

## Phases (tickets)
0. Measurement trust — pretender-6tt (Rust `params` never captured → params_max
   unenforced on our own code), pretender-gv5 (void_mutator flapping → CI flake
   risk now that the gate is in CI), .1 (baseline hygiene: `just dogfood-adopt` +
   the "refactor PRs ship a refreshed baseline" agreement).
1. File size — .2 main.rs, .3 engine.rs, .4 cli_test.rs, .5 the remaining
   >400-line files.
2. Function length — .6 main/engine, .7 mid+small modules, .8 tests.
3. Complexity — .9 cyclomatic/cognitive, .10 nesting, .11 abc.
4. Error handling — .12 no unwrap/expect in app code.
5. Leftovers — .13 excessive_fields, .14 mock_count/lazy_cluster.
6. The ratchet — .15 tighten pretender.toml + empty the baseline.
7. Documentation — .16 CONTRIBUTING section.

## Working agreements
- Pure refactor per ticket: no behavior change, tests green (pass-to-pass).
- One concern per ticket, PR under 400 changed lines.
- Tidy First: RED (gate reports the violation / a failing test) → GREEN → REFACTOR.
- Thresholds only move in the tightening direction, and only in .15.
- The tool's rule defaults are untouched (that would need an openspec proposal).

## Known trap
Baseline finding IDs are `file::symbol::rule`; renaming or splitting a function
turns a baselined finding into a "new" one and fails the gate (verified in a
scratch project). Hence .1: every refactor PR refreshes the baseline, and the
reviewer reads that diff to confirm it is only renames/moves.

