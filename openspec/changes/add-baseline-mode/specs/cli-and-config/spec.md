## ADDED Requirements

### Requirement: Baseline CLI Subcommands

The system SHALL provide a `pretender baseline` subcommand group with sub-subcommands `create`,
`update`, and `show`. Each MUST be accessible as `pretender baseline <subcommand> [args]`.

#### Scenario: baseline create is invoked

- **WHEN** `pretender baseline create` is run
- **THEN** it runs the full check pipeline, writes `pretender.baseline.json`, and exits with code 0

#### Scenario: baseline update is invoked

- **WHEN** `pretender baseline update` is run
- **THEN** it re-runs the check pipeline, replaces `pretender.baseline.json`, and exits with code 0

#### Scenario: baseline show is invoked

- **WHEN** `pretender baseline show` is run
- **THEN** it prints a table of all grandfathered findings to stdout and exits with code 0

---

### Requirement: Baseline Flag on Check

The system SHALL accept a `--baseline` flag on `pretender check`. When supplied, the check MUST
compare each finding against the loaded baseline file and exit non-zero only for findings that
are absent from the baseline or have regressed beyond their baselined value.

#### Scenario: check --baseline passes on grandfathered finding

- **WHEN** `pretender check --baseline` is run and a violation matches a baseline entry with a value not exceeding the baselined value
- **THEN** that finding does not contribute to a non-zero exit code

#### Scenario: check --baseline fails on new finding

- **WHEN** `pretender check --baseline` is run and a violation has no matching baseline entry
- **THEN** the command exits with a non-zero code and reports the new finding

#### Scenario: check --baseline fails on regression

- **WHEN** `pretender check --baseline` is run and a finding's value has increased beyond its bucketed baseline value
- **THEN** the command exits with a non-zero code and reports the regression

---

### Requirement: Baseline Configuration Table

The system SHALL support a `[baseline]` table in `pretender.toml` with the following keys:

| Key                   | Type    | Default                      | Description                                     |
|-----------------------|---------|------------------------------|-------------------------------------------------|
| `path`                | string  | `"pretender.baseline.json"`  | Path to the baseline snapshot file              |
| `auto_update_improved`| boolean | `true`                       | Tighten baseline on improved findings without stderr output or exit-code change (logs `baseline.tightened` at INFO) |

#### Scenario: Custom baseline path is honored

- **WHEN** `[baseline] path = ".pretender/baseline.json"` is set in `pretender.toml`
- **THEN** all baseline subcommands and `check --baseline` read from and write to that path

#### Scenario: auto_update_improved disabled prevents baseline tightening

- **WHEN** `[baseline] auto_update_improved = false` is set
- **THEN** improved findings do not modify the baseline file during `pretender check --baseline` and no `baseline.tightened` log entry is emitted
