## ADDED Requirements

### Requirement: Inheritance Depth Threshold

The system SHALL accept an `inheritance_depth_max: u32` field under `[thresholds]` in `pretender.toml`, defaulting to `0` (disabled). When enabled, code units (classes) with inheritance chain depth greater than the threshold SHALL be flagged.

#### Scenario: Inheritance depth exceeds threshold
- **WHEN** a class extends `B` which extends `C` which extends `D` all in the same file (depth = 3) and `inheritance_depth_max = 2`
- **THEN** the system emits an `inheritance_depth` violation for that class

#### Scenario: Inheritance depth within threshold
- **WHEN** a class extends `B` (depth = 1) and `inheritance_depth_max = 2`
- **THEN** the system emits no violation

#### Scenario: Zero threshold disables the check
- **WHEN** `inheritance_depth_max = 0`
- **THEN** no inheritance-depth analysis is performed

#### Scenario: No parent class has depth 0
- **WHEN** a class has no `extends`/`superclass` declaration
- **THEN** depth = 0 and no violation is emitted

#### Scenario: Rust stdlib trait bound excluded
- **WHEN** a Rust file has `trait A: Display + Debug`
- **THEN** the system emits no violation (stdlib traits are not counted as inheritance)

#### Scenario: Cross-file parent class depth limited
- **WHEN** a class extends `ImportedBase` defined in another file
- **THEN** depth = 1 with a diagnostic note that full chain is unknown

#### Scenario: Generated and vendor files excluded
- **WHEN** a file has role `generated` or `vendor`
- **THEN** the system performs no inheritance-depth analysis for that file

### Requirement: Inheritance Depth Violation Output

The system SHALL emit `inheritance_depth` violations in `human`, `json`, and `sarif` output formats, including depth count, threshold, and the parent-class chain.

#### Scenario: SARIF rule ID
- **WHEN** `pretender check --format sarif` runs on a file with inheritance-depth violations
- **THEN** the SARIF output includes results with rule id `pretender/inheritance-depth`