# Issue review (Tracer-Bullet, Rule of 5) — structural-clarity issue set

Scope: 33 issues (epic pretender-5zo + 22 children, epic pretender-b5us + 6
children, pretender-6tt, pretender-gv5, pretender-1eua).
Source plan: wai project `structural-clarity` (plan + design + RO5U research).

## PASS 0 (pre-flight, mechanical)
- PRE-001 MEDIUM x31: 31/33 tickets lack `metadata.files`; 31/33 lack `base_commit`.
- PRE-002 (masked by PRE-001, manual): .7/.9/.10/.11 all claim the same ~15-20
  source files -> shared-file churn; PRE-002 would be HIGH once files are declared.
- PRE-005 MEDIUM x33: no `estimate` on any ticket (session fit unknown).
- PRE-006: vacuously clean (no estimates).
- PRE-007 LOW: pretender-uwz claimed (out of scope, pre-existing).
- Lane labels (`afk`/`design`/`hitl`/`review`) missing on 31 tickets.

## PASS 1 (clarity) — GOOD
Why/Scope/Acceptance present on nearly every ticket. Gap: the *control-surface*
tickets (.15, .18-.22) carry no runnable command; .15 (the ratchet — the ticket
the whole epic exists for) has no Must gate at all.

## PASS 2 (scope/shape) — the headline finding
The epic is decomposed **horizontally by rule**: .6/.7 function_lines, .9
cyclomatic/cognitive, .10 nesting, .11 abc, .12 unwraps. The same files are
therefore edited five times across five phases, and tickets are blocked by
decomposition artifacts rather than real prerequisites (SCOPE-001, HIGH).
Also: .2 (main.rs, 3977 lines), .3 (engine.rs, 1749), .4 (cli_test.rs, 3900),
.5 (six files) are epic-sized but tagged `complexity:l` (SCOPE-002, HIGH) — they
are decomposition candidates, not schedulable tickets. .19 (five gates in one
ticket) is likewise >1 session (SCOPE-003, MEDIUM).

Recommendation: reslice to **per-file vertical slices** — "make module X fully
clean" (all rules for that module in one ticket, one PR) — and split the giants
into one-PR children. The rule checklist moves *inside* each ticket; the phase
map stays as a metric dashboard, not as a work order.

## PASS 3 (dependencies) — GOOD, two gaps
No cycles (`bd dep cycles`). DEP-001 MEDIUM: phase-0 trust items (6tt params
capture, gv5 flapping) block only .2-.5; .6-.14 could "clean" code that was
never measured for params. DEP-002 LOW: .20 (golden) blocks .6-.8/.12-.14 but
not .2-.5 (pure moves) — inconsistent; keep .20 blocking only behavior-touching
tickets.

## PASS 4 (alignment) — GOOD
Tickets trace to the epic plan; the plan is binding and was rewritten in the
same session. Gap: no ticket carries a plan/spec reference (`--spec-id`), so
traceability relies on the parent link alone (ALIGN-001, LOW).

## PASS 5 (executability)
- EXEC-001 HIGH: no file-header criterion (Purpose/Responsibilities/Rationale)
  on any ticket that creates or reshapes source files (.2-.13 create modules).
  Repo convention already exists (see pretender-6tt notes).
- EXEC-002 HIGH: no quantified Must + runnable Meter on the control-surface
  tickets (.15, .18-.22).
- EXEC-003 HIGH: .2/.3/.4/.5 mis-tagged `complexity:l` (XL in reality).
- EXEC-004 MEDIUM: lane missing; `pretender-b5us.1/.2/.3` are DESIGN/HITL work
  (dispatch research, registry shape, openspec proposal) and must not be
  scheduled as AFK.
- EXEC-005 MEDIUM: test-touching tickets (.4, .8, .14) lack anti-goals
  ("no assertion deleted, no test skipped").
- EXEC-006 MEDIUM: regression suite unnamed on the control-surface tickets.
- EXEC-007 MEDIUM: no estimates -> no session-fit signal.

## Convergence
After Pass 2: 2 HIGH, 1 MEDIUM new — CONTINUE
After Pass 3: 1 MEDIUM, 1 LOW new — CONTINUE
After Pass 5: 3 HIGH, 4 MEDIUM new — CONTINUE (converged on the same two themes:
horizontal slicing, missing machine gates)

## Verdict
NEEDS_UPDATES — the epic structure is right; the tickets need reslicing,
metadata, lane/complexity tags, estimates, file-header criteria, anti-goals and
runnable Must gates before they can be scheduled.

