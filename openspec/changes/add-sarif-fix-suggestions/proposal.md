# Change: Add SARIF Fix Suggestions

## Why

Pretender already emits valid SARIF 2.1.0 for findings, but the output omits the `fixes` array available in the SARIF schema. Without fix suggestions, findings appear as bare annotations in IDEs and PR comments — developers see the problem but have no starting point for action. Emitting `fixes` blocks enables one-click suppression in SARIF-aware IDEs (VS Code SARIF Viewer, Rider, GitHub Code Scanning) and makes findings actionable at the point of review.

## What Changes

- For each rule that has a mechanical fix, emit a SARIF `fixes` array in the `result` object when output format is `sarif`
- Fix strategies per rule:
  - `min_assertions` → insert `// TODO: pretender: this test has no assertions` at function top
  - `params_max` → insert `// pretender: consider grouping params {a,b,...} into a struct`
  - `function_lines_max` → comment naming the largest nested block as an extraction candidate
  - `cyclomatic_max` / `cognitive_max` → comment pointing to the highest-weight branch(es)
  - `duplication` → comment on both clone sides referencing the other location
- Each fix carries the rule id so suppressions can be targeted
- Rules with no mechanical fix: no `fixes` block emitted (absent is valid SARIF)

## Impact

- Affected specs: `cli-and-config` (SARIF output format extended with fixes)
- Affected code: SARIF emitter, per-rule result builders
- Dependencies: requires `update-mvp-spec-baseline` to be applied first and requires a SARIF output implementation to restore `--format sarif` before fix suggestions can be emitted
- No breaking changes — `fixes` is an optional SARIF field; existing consumers ignore it
