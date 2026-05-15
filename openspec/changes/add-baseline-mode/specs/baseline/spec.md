## ADDED Requirements

### Requirement: Baseline Snapshot Creation

The system SHALL provide `pretender baseline create` to snapshot the current set of rule violations
into a file (default: `pretender.baseline.json`). The snapshot MUST include, for each finding, a
stable identity fingerprint derived from `(file, unit_name, unit_start_line, rule)`, the rule name, the file path,
the unit name, the unit start line, the raw metric value, the configured threshold, and a coarse bucket value for regression comparison.

#### Scenario: Create writes baseline file

- **WHEN** `pretender baseline create` is invoked on a repository with existing violations
- **THEN** `pretender.baseline.json` is written containing one entry per violation with its fingerprint, rule, file, unit, unit start line, value, threshold, and bucket

#### Scenario: Create on clean repo writes empty findings

- **WHEN** `pretender baseline create` is invoked and no violations exist
- **THEN** `pretender.baseline.json` is written with `"findings": []`

---

### Requirement: Baseline Update

The system SHALL provide `pretender baseline update` to replace the existing baseline snapshot
with the current state of violations. Entries for units that no longer exist MUST be pruned.

#### Scenario: Update replaces previous snapshot

- **WHEN** `pretender baseline update` is invoked after violations have been fixed
- **THEN** the baseline file is overwritten with only the currently detected violations

#### Scenario: Deleted unit entries are pruned

- **WHEN** a function referenced in the baseline no longer exists in the codebase
- **THEN** `pretender baseline update` removes its entry from the baseline file

---

### Requirement: Baseline Show

The system SHALL provide `pretender baseline show` to list all grandfathered findings currently
recorded in the baseline file in a human-readable table. The command MUST support
`--format json` for machine-readable output.

#### Scenario: Show lists grandfathered findings

- **WHEN** `pretender baseline show` is invoked and the baseline file contains entries
- **THEN** each entry is printed with its file, unit, rule, value, and threshold

#### Scenario: Show on missing file reports error

- **WHEN** `pretender baseline show` is invoked but no baseline file exists
- **THEN** the command exits with a non-zero code and prints a descriptive error message

---

### Requirement: Baseline Fingerprinting

The system SHALL derive the fingerprint for each baseline entry as the SHA-256 hash of
`file_path + NUL + unit_name + NUL + unit_start_line + NUL + rule`. The fingerprint identifies the finding independent
of its current metric value while disambiguating repeated or nested units with the same name in one file. The baseline entry MUST also store a `bucket` value computed as
`floor(value / max(1, threshold / 5))`, producing coarse buckets so that minor metric fluctuations
within the same bucket do not count as regressions.

Preconditions for the bucket formula: `value` MUST be a finite, non-negative real number and `threshold` MUST be a finite, strictly positive real number. The division and `max` operations MUST be evaluated in IEEE-754 double precision and the `floor` result MUST be truncated toward zero to a non-negative integer. The system SHALL reject any rule whose configured threshold is `<= 0`, NaN, or infinite at config-load time.

#### Scenario: Same bucket value matches fingerprint

- **WHEN** a function's metric value changes but its file, unit name, unit start line, and rule are unchanged
- **THEN** the fingerprint is identical and the finding is considered to match the baseline entry

#### Scenario: Bucket increase is a regression

- **WHEN** a function's metric value increases enough to move it above the stored baseline bucket
- **THEN** the fingerprint still matches but the finding is treated as a regression

---

### Requirement: Baseline Ratchet

When `auto_update_improved = true` (the default), the system SHALL tighten a baseline entry
whenever the actual metric value moves into a lower bucket than the stored baseline bucket. The updated entry MUST use
the new lower value and new lower bucket while preserving the stable identity fingerprint. The tightening MUST NOT
produce stderr output, MUST NOT alter the process exit code, and MUST NOT emit a SARIF result; it SHALL be recorded
only via a structured-log entry at `INFO` level with event name `baseline.tightened`. This prevents re-introduction of
previously grandfathered bucket ranges without triggering a failure.

Monotonicity invariant: for any fingerprint `F` present in the baseline at times `t1 < t2` with no intervening `pretender baseline create` or `pretender baseline update` command, `bucket(F, t2) <= bucket(F, t1)`. The ratchet path MUST NOT raise a stored `bucket` and MUST NOT mutate the stored `fingerprint`; only `pretender baseline create` and `pretender baseline update` may raise a stored `bucket` (by replacing the snapshot wholesale).

#### Scenario: Improvement tightens baseline without user-visible output

- **WHEN** `pretender check --baseline` is run and a finding's value falls into a lower bucket than its stored baseline bucket
- **THEN** the baseline entry is updated to the new lower value and bucket, the command exits with code 0 (assuming no other failing findings), stderr is empty for this event, and the structured log contains exactly one `baseline.tightened` entry for the affected fingerprint

#### Scenario: Re-introduction of grandfathered value fails

- **WHEN** a baseline entry has been tightened to value 18 and the function is later edited to value 23
- **THEN** `pretender check --baseline` exits with a non-zero code for that finding

#### Scenario: Ratchet never loosens within a stored bucket

- **WHEN** `pretender check --baseline` is run, a finding's value increases but stays within the bucket already stored in the baseline
- **THEN** the stored `bucket` and stored `value` are both unchanged (no widening), the structured log contains zero `baseline.tightened` entries for the affected fingerprint, and the command exits with code 0 (assuming no other failing findings)

---

### Requirement: Baseline File Format

The baseline file MUST be a JSON file conforming to:

```json
{
  "version": 1,
  "created_at": "<ISO-8601 timestamp>",
  "findings": [
    {
      "fingerprint": "sha256:<hex>",
      "rule": "<rule-id>",
      "file": "<relative-path>",
      "unit": "<unit-name>",
      "unit_start_line": <integer>,
      "value": <number>,
      "threshold": <number>,
      "bucket": <integer>
    }
  ]
}
```

The `version` field MUST be `1` for this revision. The system SHALL reject files with unknown
`version` values and report a clear error.

#### Scenario: Valid baseline file is parsed correctly

- **WHEN** a `pretender.baseline.json` with `"version": 1` exists
- **THEN** the system loads all findings and uses them for baseline comparison

#### Scenario: Unknown version is rejected

- **WHEN** a `pretender.baseline.json` with `"version": 99` is present
- **THEN** the system exits with a non-zero code and reports that the baseline format version is unsupported
