# Design: the enforced control surface (RO5U decisions, accepted)

Decision record for the review of 2026-11-24. All five proposals accepted.

## 1. The baseline IS the burn-down ledger (keystone)
Old model: the baseline grandfathers everything; each refactor PR runs
`--adopt` to refresh it. That model has no RED (the gate is green) and its
refresh step is exactly the gaming vector.

New model:
- RED   = REMOVE the ticket's finding IDs from `pretender-baseline.json`
          (un-grandfather) → `just dogfood` fails with exactly those findings.
- GREEN = refactor until the gate passes.
- TIDY  = renames/dead code, separate commit.
- The baseline may only SHRINK. Additions are forbidden inside the epic; the
  single exception is the ratchet ticket, which re-adopts deliberately once the
  whole class is clean.
- Why this is also anti-gaming: rename-gaming (rename a function, then adopt the
  "new" ID) is impossible, because the ID set can only get smaller.

## 2. Mechanical gates (CI, not goodwill)
| gate | blocks | mechanism |
|---|---|---|
| structural gate | new violations | `just dogfood` (existing) |
| baseline monotonicity | adopt-gaming, new-debt baselining | current IDs ⊆ `origin/main` IDs |
| baseline purity | a ratchet that achieves nothing | no IDs under `pretender/src/`, `pretender/tests/` |
| threshold floor | raising a threshold, disabling a rule (`= 0` in a role table) | committed floor; current ≤ floor; floor only tightens |
| exclude allowlist | excluding `**/src/**` | exclude list equals exactly 4 known patterns |
| assertion non-regression | deleting assertions to pass | total `assertions` metric must not decrease |
| golden behavior | silent behavior change in a "pure refactor" | analyzer output over the fixture corpus, byte-identical |
| independent review | self-review | bd bead gate: implementer waits for reviewer bead |

Golden corpus caveat (established while designing): explicit paths are filtered
by `exclude` too (`check pretender/tests/fixtures` → 0 files) and there is no
`--config` flag — so the recipe copies the corpus to a temp dir outside the repo
(no config discovered → default excludes) and diffs against the committed
golden. No product change needed.

## 3. Per-ticket contract and pipeline
Every implementation ticket: research → design → RED → GREEN → TIDY →
independent review → mechanical gates → ledger update, run inside
`wai pipeline start tdd-ro5u --topic="<ticket>"`. The pipeline runs at TICKET
level; the epic-level run is the wrong granularity.

## 4. Independence
`bd gate` types used: `bead` (reviewer bead must close), `gh:pr` (PR merged),
`human` (escalation on disagreement). Implementer ≠ reviewer (different
assignees). Reviewer's verdict is committed as a wai review artifact — that
artifact is the gate's evidence.

## 5. Cohesion / open-closed / declarative → separate epic
The open-closed violation is language dispatch (`engine.rs::parse` cyclomatic
26 / cognitive 89; eight language modules repeating manifest/engine/parse).
That is a design change, not a tidy refactor → filed as epic **pretender-b5us**
("Language registry"), with its own research → design → openspec proposal →
implementation chain. It informs the engine.rs split: the split must not freeze
the dispatch shape.

## Where this lives
- Epic description (pretender-5zo): binding working agreements.
- Tickets .18–.22: publish the contract, build the gates, wire independence.
- Tickets .1–.16: the refactor work itself.

