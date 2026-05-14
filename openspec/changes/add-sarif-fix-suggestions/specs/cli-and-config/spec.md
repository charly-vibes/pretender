## MODIFIED Requirements

### Requirement: SARIF Fix Suggestions

When output format is `sarif`, the system SHALL emit a `fixes` array in each `result` object for rules that have a defined mechanical fix strategy. The `fixes` array SHALL NOT be emitted for rules with no mechanical fix; an absent `fixes` key is valid SARIF 2.1.0.

Each fix SHALL carry the `ruleId` of the parent finding so SARIF-aware IDEs can link the fix to the suppression action.

Fix strategies by rule:

| Rule | Fix text inserted |
|------|-------------------|
| `min_assertions` | `// TODO: pretender: this test has no assertions` at line 1 of function body |
| `params_max` | `// pretender: consider grouping params {a,b,...} into a struct` at function signature |
| `function_lines_max` | `// pretender: consider extracting '<block>' (lines N–M)` above the largest nested block |
| `cyclomatic_max` | `// pretender: highest-weight branch at line N — consider extracting` |
| `cognitive_max` | `// pretender: highest-weight branch at line N — consider extracting` |
| `duplication` | `// pretender: clone of <other-file>:<line>` on the primary clone-pair result, referencing the related location |

Fixes are suggestion-only and SHALL NOT be auto-applied. They appear as IDE quick-fix options in SARIF-aware tooling.

#### Scenario: min_assertions fix emitted

- **WHEN** a finding for `min_assertions` is emitted in `sarif` format
- **THEN** the `result` object contains a `fixes` array with one entry inserting the TODO comment at the first line of the function body

#### Scenario: params_max fix lists parameter names

- **WHEN** a finding for `params_max` is emitted in `sarif` format for a function with params `(a, b, c, d, e)`
- **THEN** the fix text reads `// pretender: consider grouping params a, b, c, d, e into a struct`

#### Scenario: function_lines_max fix names extraction candidate

- **WHEN** a finding for `function_lines_max` is emitted in `sarif` format
- **THEN** the fix inserts a comment above the largest nested block naming it as an extraction candidate with its line range

#### Scenario: cyclomatic_max fix points to highest-weight branch

- **WHEN** a finding for `cyclomatic_max` is emitted in `sarif` format
- **THEN** the fix inserts a comment referencing the line of the branch that contributes the most to the cyclomatic score

#### Scenario: cognitive_max fix points to highest-weight branch

- **WHEN** a finding for `cognitive_max` is emitted in `sarif` format
- **THEN** the fix inserts a comment referencing the line of the branch with the highest cognitive weight (branch_weight × (1 + nesting_at))

#### Scenario: duplication fix cross-references related location

- **WHEN** a `duplication` finding is emitted in `sarif` format for a clone pair at locations A and B
- **THEN** the single clone-pair SARIF result includes one fix on the primary location that references the related location B

#### Scenario: nesting_max produces no fixes block

- **WHEN** a finding for `nesting_max` is emitted in `sarif` format
- **THEN** the `result` object contains no `fixes` key

#### Scenario: fixes absent from non-SARIF formats

- **WHEN** output format is `human`, `json`, `junit`, or `markdown`
- **THEN** no fix suggestion text is included in the output
