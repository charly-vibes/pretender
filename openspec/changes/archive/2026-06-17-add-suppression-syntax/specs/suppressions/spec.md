## ADDED Requirements

### Requirement: Inline Suppression Pragma Syntax

The system SHALL recognize inline comment pragmas of the form:

```
// pretender: allow[rule1, rule2] reason="..." until="YYYY-MM-DD"
```

The `reason` field is MANDATORY. The `until` field is OPTIONAL. An `allow[*]` wildcard MUST be
rejected as a validation error; rules MUST be explicitly enumerated. The suppression applies to
the next `CodeUnit` in the file, or to the entire `Module` if the pragma appears before any
`CodeUnit` (i.e., at the file top).

#### Scenario: Valid pragma is parsed

- **WHEN** a source file contains `// pretender: allow[cyclomatic] reason="intentional parser complexity"`
- **THEN** the pragma is parsed successfully and the named rule is suppressed for the following code unit

#### Scenario: Missing reason is a validation error

- **WHEN** a pragma is encountered with an empty or absent `reason` field
- **THEN** the system emits a validation error identifying the file and line of the offending pragma

#### Scenario: Wildcard allow is rejected

- **WHEN** a pragma is encountered with `allow[*]`
- **THEN** the system emits a validation error stating that wildcard suppression is forbidden

#### Scenario: File-top pragma suppresses module

- **WHEN** a pragma appears before any `CodeUnit` in a file
- **THEN** the named rules are suppressed for all findings in the entire `Module`

---

### Requirement: Suppression Expiry

The system SHALL support an optional `until="YYYY-MM-DD"` field in the suppression pragma. After
the specified date, the suppression MUST be ignored and any underlying violation MUST resurface as
a normal finding. An expired suppression MUST NOT cause a hard error; the system SHALL emit a
warning identifying the expired pragma and its location.

#### Scenario: Active expiry suppresses violation

- **WHEN** a pragma has `until="2099-12-31"` and today is before that date
- **THEN** the suppression is active and the associated rule is suppressed for that code unit

#### Scenario: Expired suppression restores violation

- **WHEN** a pragma has `until="2020-01-01"` and today is after that date
- **THEN** the suppression is ignored, the violation resurfaces, and a warning is emitted identifying the expired pragma

---

### Requirement: Suppression Scope Resolution

The system SHALL resolve suppression scope during CST traversal before metric computation.
A pragma MUST be attached to the immediately following `CodeUnit`. Only one `CodeUnit` per
pragma is suppressed; the pragma does not carry forward to subsequent units.

#### Scenario: Suppression attaches to next unit only

- **WHEN** a pragma precedes two consecutive functions in a file
- **THEN** only the first function's rules are suppressed; the second function is evaluated normally

#### Scenario: Suppression does not affect other rules

- **WHEN** `allow[cyclomatic]` is present on a code unit
- **THEN** `cognitive`, `nesting`, and all other rules are still evaluated for that unit

---

### Requirement: Suppression Engine Integration

The system SHALL skip rule evaluation for a `CodeUnit` or `Module` that carries an active, non-expired
suppression matching the rule being evaluated. Suppressed findings MUST NOT appear in check results
or contribute to exit code decisions.

#### Scenario: Suppressed finding absent from check output

- **WHEN** `pretender check` is run on a file with an active suppression for `cyclomatic` on a function
- **THEN** that function's cyclomatic complexity finding does not appear in the output and does not affect the exit code

#### Scenario: Non-suppressed rules still reported

- **WHEN** a function has an active `allow[cyclomatic]` suppression but also exceeds `cognitive_max`
- **THEN** the cognitive complexity finding is still reported and contributes to the exit code
