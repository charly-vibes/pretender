# RO5U review: the structural-clarity investigation (epic pretender-5zo + the loop/heuristics)

Artifact under review: the epic plan (`plans/2026-11-24-epic-plan-…`), the beads
epic pretender-5zo + its 17 tickets, and the loop/heuristics proposed in session
(loop is currently only in conversation — that is finding DRAFT-001).

Lens requested by the requester: (a) gates that block unwanted changes and
agent gaming, (b) independent implementer/reviewer, (c) explicit research/design
before implementing, with tdd-ro5u at ticket level, (d) cohesive, open/closed,
declarative code.

## STAGE 1 — DRAFT
Shape: FAIR. The phase map and the measurement are sound; the *control surface*
is prose. Every commitment that must survive contact with an autonomous agent
("thresholds only move tighter", "reviewer reads the baseline diff", "no
behavior change", "pure refactor") has no mechanical counterpart.

[DRAFT-001] HIGH — No single source of truth. The plan lives in three places
(wai artifact, 18 beads issues, and chat-only heuristics) with no links and no
statement of which is authoritative. A future agent will read the tickets and
miss the loop.
[DRAFT-002] HIGH — No gates. The plan is a policy document, not an enforced
system. Nothing in CI or in a test prevents: adopting new debt into the
baseline, raising a threshold, widening `exclude`, disabling a rule for a path,
or deleting an assertion.
[DRAFT-003] MEDIUM — Cohesion / open-closed / declarative are absent. "Split
into focused modules" has no responsibility model, and the plan never names the
actual open-closed violation: the language dispatch explosion (`engine.rs::parse`
cyclomatic 26 / cognitive 89, per-language `manifest/engine/parse` triplication
in c.rs, go.rs, java.rs, javascript.rs, python.rs, ruby.rs, rust.rs,
typescript.rs…). Splitting files without changing the dispatch shape yields
cohesive-looking files around an unchanged design.
[DRAFT-004] MEDIUM — Implementer/reviewer independence is not in the plan at all:
no assignee separation, no gate type, no artifact trail, no arbitration path.

## STAGE 2 — CORRECTNESS
[CORR-001] CRITICAL — RED is fictional for all 164 findings. The plan claims
"Tidy First: RED (gate reports the violation)". The gate is *green*: the
committed baseline grandfathers every finding (`→ gate: 164 known (baselined), 0
new`, exit 0). There is therefore no failing test for any epic ticket — Tidy
First would be enacted with no RED, and the loop's first step is a no-op.
[CORR-002] CRITICAL — The ratchet ticket (.15) can pass while achieving nothing:
`--adopt` alone turns the gate green whatever the code looks like. The
acceptance criterion ("baseline contains no production-code findings") is prose.
[CORR-003] HIGH — Threshold direction is unenforced: nothing stops an agent
raising `function_lines_max` from 30 to 40 to silence a finding.
[CORR-004] HIGH — Exclusions can be widened silently: nothing stops
`exclude = ["**/src/**"]`.
[CORR-005] HIGH — Behavior preservation is asserted, not measured. The suite
does not cover the logic being extracted (e.g. `analyze_path`'s report
assembly); a refactor can change output and stay green.
[CORR-006] MEDIUM — Number drift: the plan says "16 child tickets" (now 17 +
pretender-1eua) and "174 violations" (now 164, fixtures excluded). Hand-
maintained numbers will keep drifting.
[CORR-007] MEDIUM — Phase-0 ordering is prose: only tickets .2–.5 depend on the
params fix (pretender-6tt); .6–.14 could "clean" code that was never measured
for params.

## STAGE 3 — CLARITY
[CLAR-001] MEDIUM — "Independent implementer/reviewer" has no operational
definition: who reviews, in which artifact, with which authority, and how
disagreement is escalated.
[CLAR-002] MEDIUM — "Cohesive" is not checkable: no responsibility-map
artifact is required per split ticket.
[CLAR-003] LOW — "Working agreements" mixes hard gates with preferences; a
reader cannot tell which are enforced.

## STAGE 4 — EDGE CASES
[EDGE-001] HIGH — Rename-gaming: an agent renames a function so its finding
becomes "new", then adopts it. The baseline diff looks like a rename, and the
current gate stays green.
[EDGE-002] MEDIUM — Scope-gaming: disabling a rule for a path
(`[thresholds.test] cyclomatic_max = 0`) achieves the same as raising the
threshold; a floor check that only inspects the app table misses it.
[EDGE-003] MEDIUM — Test-weakening gaming: "no assertion may be deleted" is
prose only.
[EDGE-004] MEDIUM — Self-review: the same agent can implement and review its
own ticket; the plan has no mechanism to prevent it.
[EDGE-005] LOW — No arbitration path when an independent reviewer rejects.

## STAGE 5 — EXCELLENCE
The review converges on one missing spine: **a declarative epic state file plus
a per-ticket contract**. Everything the requester asked for (gates, independence,
research/design-before-implementation, declarative/OCP shaping) is a
consequence of those two artifacts existing. Proposed in the session summary:
`epic-state.toml` (targets + burn-down ledger + phase gates), the ticket
contract (research → design → RED → GREEN → independent review → mechanical
gates → ledger update), and eight machine-checkable gates, six of which are
cheap to add as governance tests next to `repo_dogfood_test.rs`.

## Convergence
- Stage 2: 2 CRITICAL, 3 HIGH, 2 MEDIUM new — CONTINUE
- Stage 3: 3 MEDIUM, 1 LOW new — CONTINUE
- Stage 4: 1 HIGH, 3 MEDIUM, 1 LOW new — CONTINUE
- Stage 5: converged (the spine explains all findings)

## Verdict
NEEDS_REWORK — of the epic *definition*, before implementation starts. The
phase map is right; the control surface is missing. The rework is cheap (one
declarative file + governance tests + a ticket contract) and is what makes the
rest of the epic safe to delegate to agents.

