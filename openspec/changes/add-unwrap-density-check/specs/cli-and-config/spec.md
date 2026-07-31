## ADDED Requirements

### Requirement: Unwrap Density Threshold

The system SHALL accept an `unwrap_max: u32` field under `[thresholds]` in `pretender.toml`, defaulting to `0` (disabled). Role-specific overrides SHALL be supported via `[thresholds.<role>] unwrap_max`. When enabled, code units with more unwrap/except/catch calls than the threshold SHALL be flagged.

#### Scenario: Unwrap count exceeds threshold
- **WHEN** a Rust file has 7 `unwrap()` calls and `unwrap_max = 5`
- **THEN** the system emits an `unwrap` violation for that code unit

#### Scenario: Bare except in Python
- **WHEN** a Python file has a bare `except:` clause and `unwrap_max = 1`
- **THEN** the system emits an `unwrap` violation for the enclosing function

#### Scenario: Unwrap count within threshold
- **WHEN** a code unit has 3 `unwrap()` calls and `unwrap_max = 5`
- **THEN** the system emits no violation

#### Scenario: Zero threshold disables the check
- **WHEN** `unwrap_max = 0`
- **THEN** no unwrap-density analysis is performed

#### Scenario: Role-specific override
- **WHEN** `[thresholds.app] unwrap_max = 0` and `[thresholds.test] unwrap_max = 5`
- **THEN** app-role files must have zero unwraps, test-role files allow up to 5

#### Scenario: Generated and vendor files excluded
- **WHEN** a file has role `generated` or `vendor`
- **THEN** the system performs no unwrap-density analysis for that file

#### Scenario: Go idiomatic error propagation excluded
- **WHEN** a Go file contains `if err != nil { return err }`
- **THEN** the system emits no finding (this is idiomatic Go)

### Requirement: Unwrap Violation Output

The system SHALL emit `unwrap` violations in `human`, `json`, and `sarif` output formats, including unwrap count, threshold, and per-call-site locations.

#### Scenario: SARIF rule ID
- **WHEN** `pretender check --format sarif` runs on a file with unwrap violations
- **THEN** the SARIF output includes results with rule id `pretender/unwrap-density`