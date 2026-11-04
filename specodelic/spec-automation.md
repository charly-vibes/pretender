---
id: spec.automation
kind: intent
statement: "THE system SHALL keep every deployed scenario in openspec/specs covered by at least one contract entry under .espectacular that names a runnable test."
---

# automation

Pretender's spec quality pipeline is two-layer: OpenSpec owns the scenario
prose (`openspec/specs/**`), and espectacular owns the correspondence between
those scenarios and runnable tests (`.espectacular/<spec>/<scenario>.toml`).
This spec pins the invariant that makes the pre-commit gates meaningful: a
deployed scenario with no contract entry is unverifiable prose, and a contract
entry naming a test that does not exist is a lie the gate must catch. It does
not cover change-overlay scenarios (they are governed by their change until
archived into `openspec/specs/`).

## Constraints

| id | kind | expr | traces_to |
|----|------|------|-----------|
| c1 | invariant | every scenario id under openspec/specs has a non-empty `.espectacular/<spec>/<scenario>.toml` whose test entries name tests that exist in `file` | [[spec.automation]] |

## Model

### States

- `initial`

### Transitions

| id | from | to | guard |
|----|------|----|-------|
| t1 | initial | initial | [[spec.automation.c1]] |

## Properties

| id | kind | derives_from | generator | predicate |
|----|------|--------------|-----------|-----------|
| p1 | unit | [[spec.automation.c1]] | any deployed scenario id from `ah check --json` | `ah check` reports no `no-tests-declared` and no `orphan-toml` finding for that id |
