## ADDED Requirements

### Requirement: Mutable-State Ratio Threshold

The system SHALL accept a `mut_ratio_max: f64` field under `[thresholds]` in `pretender.toml`, defaulting to `0.0` (disabled). When enabled, files with a mutable-binding ratio exceeding this value SHALL be flagged.

#### Scenario: Mutable ratio exceeds threshold
- **WHEN** a file has 12 mutable bindings out of 20 total bindings (ratio = 0.6) and `mut_ratio_max = 0.5`
- **THEN** the system emits a mutable-state finding for that file

#### Scenario: Mutable ratio within threshold
- **WHEN** a file has ratio 0.4 and `mut_ratio_max = 0.5`
- **THEN** the system emits no finding

#### Scenario: Zero threshold disables the check
- **WHEN** `mut_ratio_max = 0.0`
- **THEN** no mutable-state analysis is performed

### Requirement: Mutable-State Finding Output

The system SHALL emit mutable-state findings in `human`, `json`, and `sarif` output formats with the finding name `mut_ratio`, including the actual ratio, threshold, and total binding counts per file.

#### Scenario: Human format includes ratio details
- **WHEN** `pretender check --format human` runs on a file with a mutable-state finding
- **THEN** the output includes the file path, actual ratio, threshold, and binding counts

#### Scenario: SARIF rule ID
- **WHEN** `pretender check --format sarif` runs on a file with a mutable-state finding
- **THEN** the SARIF output includes results with rule id `pretender/mutable-ratio`

#### Scenario: Generated and vendor files excluded
- **WHEN** a file has role `generated` or `vendor`
- **THEN** the system performs no mutable-state analysis for that file

#### Scenario: Python files excluded
- **WHEN** a file has language `Python`
- **THEN** the system performs no mutable-state analysis for that file