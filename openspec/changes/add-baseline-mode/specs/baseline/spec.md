## ADDED Requirements

### Requirement: Baseline Snapshot Creation

The system SHALL provide `pretender baseline create` to snapshot the current set of rule violations
into a file (default: `pretender.baseline.json`). The snapshot MUST include, for each finding, a
stable identity fingerprint derived from `(file, unit_name, rule)`, the rule name, the file path,
the unit name, the raw metric value, the configured threshold, and a coarse bucket value for regression comparison.

#### Scenario: Create writes baseline file

- **WHEN** `pretender baseline create` is invoked on a repository with existing violations
- **THEN** `pretender.baseline.json` is written containing one entry per violation with its fingerprint, rule, file, unit, value, and threshold

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
`file_path + NUL + unit_name + NUL + rule`. The fingerprint identifies the finding independent
of its current metric value. The baseline entry MUST also store a `bucket` value computed as
`floor(value / max(1, threshold / 5))`, producing coarse buckets so that minor metric fluctuations
within the same bucket do not count as regressions.

#### Scenario: Same bucket value matches fingerprint

- **WHEN** a function's metric value changes but its file, unit name, and rule are unchanged
- **THEN** the fingerprint is identical and the finding is considered to match the baseline entry

#### Scenario: Bucket increase is a regression

- **WHEN** a function's metric value increases enough to move it above the stored baseline bucket
- **THEN** the fingerprint still matches but the finding is treated as a regression

---

### Requirement: Baseline Ratchet

When `auto_update_improved = true` (the default), the system SHALL silently tighten a baseline entry
whenever the actual metric value moves into a lower bucket than the stored baseline bucket. The updated entry MUST use
the new lower value and new lower bucket while preserving the stable identity fingerprint. This prevents re-introduction of previously grandfathered
bucket ranges without triggering a failure.

#### Scenario: Improvement silently tightens baseline

- **WHEN** `pretender check --baseline` is run and a finding's value falls into a lower bucket than its stored baseline bucket
- **THEN** the baseline entry is updated to the new lower value and bucket without user intervention

#### Scenario: Re-introduction of grandfathered value fails

- **WHEN** a baseline entry has been tightened to value 18 and the function is later edited to value 23
- **THEN** `pretender check --baseline` exits with a non-zero code for that finding

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
