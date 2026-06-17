## ADDED Requirements

### Requirement: Explain Finding by Location

The system SHALL extend `pretender explain` to accept a `<file>::<function>` argument that prints a finding-specific explanation using data from the most recent `pretender check` cache.

The output SHALL include four sections:

1. **Rule** — rule name, definition, threshold citation (existing behavior)
2. **This finding** — file path, line span, function name, actual score, zone (green / yellow / red)
3. **Top contributors** — up to 5 nodes ranked by contribution value, each showing line number, node description, and contribution
4. **What helps** — 2-3 mechanical suggestions and the language-appropriate suppression pragma template

The most recent `pretender check` cache entry MUST include an explainable findings index containing stable finding ids, finding locations, rule ids, zone labels, top contributor traces, and the suppression pragma template required by this command. When no cache entry exists for the specified location, the system SHALL exit non-zero and print: `No cached results for '<arg>'. Run 'pretender check' first.`

#### Scenario: Explain by file and function

- **WHEN** `pretender explain src/router.rs::handle_request` is run with a warm cache entry for that function
- **THEN** the output contains all four sections: rule definition, actual score with zone, top contributing nodes, and remediation suggestions

#### Scenario: Zone label reflects threshold bands

- **WHEN** the actual score falls above the `yellow` band threshold
- **THEN** the "This finding" section shows `zone: red`

#### Scenario: No cache entry for location

- **WHEN** `pretender explain src/router.rs::handle_request` is run and no cache entry exists for that function
- **THEN** the command exits non-zero and prints: "No cached results for 'src/router.rs::handle_request'. Run 'pretender check' first."

---

### Requirement: Explain Finding by ID

The system SHALL accept a finding id (SARIF result id, e.g., `PRT-CYCLO-abc123`) as the argument to `pretender explain`. The command SHALL resolve the id to its cached location and produce the same four-section output as the location form.

#### Scenario: Explain by finding id

- **WHEN** `pretender explain PRT-CYCLO-abc123` is run and the id maps to a cached finding
- **THEN** the output is identical to running `pretender explain <file>::<function>` for that finding's location

#### Scenario: Unknown finding id

- **WHEN** `pretender explain PRT-CYCLO-unknown` is run and the id is not in the cache
- **THEN** the command exits non-zero and prints: "No cached results for 'PRT-CYCLO-unknown'. Run 'pretender check' first."

---

### Requirement: Explain Backward Compatibility

The system SHALL preserve existing behavior when `pretender explain <metric>` is called with a bare metric name (no `::` separator, no finding id prefix). The four-section finding output SHALL NOT be shown; only the metric definition, threshold, and citation SHALL be printed.

#### Scenario: Bare metric name still works

- **WHEN** `pretender explain cyclomatic` is run
- **THEN** the output contains the cyclomatic definition, threshold, and McCabe citation — and no "This finding" or "Top contributors" sections

---

### Requirement: Top Contributors Output

The "Top contributors" section SHALL list up to 5 nodes sorted by contribution value descending, each row showing: line number, node description (branch kind or call name), and numeric contribution to the metric score.

#### Scenario: Top contributors sorted descending

- **WHEN** a cyclomatic finding has branches contributing values 3, 1, 2, 4, 1
- **THEN** the top contributors list shows them in order: 4, 3, 2, 1, 1

#### Scenario: Fewer than 5 contributors

- **WHEN** a finding has only 2 contributing nodes
- **THEN** the top contributors list shows exactly 2 entries without padding
