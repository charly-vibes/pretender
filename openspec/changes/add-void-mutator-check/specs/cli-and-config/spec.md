## ADDED Requirements

### Requirement: Void Mutator Threshold

The system SHALL accept a `void_mutators_max: u32` field under `[thresholds]` in `pretender.toml`, defaulting to `0` (disabled). Role-specific overrides SHALL be supported via `[thresholds.<role>] void_mutators_max`. When enabled, files with more void-return mutation methods than the threshold SHALL be flagged.

#### Scenario: Void mutator count exceeds threshold
- **WHEN** a file has 5 void methods that mutate `self`/`this` fields and `void_mutators_max = 3`
- **THEN** the system emits a `void_mutator` violation for that file

#### Scenario: Void mutator count within threshold
- **WHEN** a file has 2 void mutators and `void_mutators_max = 3`
- **THEN** the system emits no violation

#### Scenario: Zero threshold disables the check
- **WHEN** `void_mutators_max = 0`
- **THEN** no void-mutator analysis is performed

#### Scenario: Role-specific override
- **WHEN** `[thresholds.test] void_mutators_max = 5` overrides the global default of 3
- **THEN** test-role files use threshold 5

#### Scenario: Generated and vendor files excluded
- **WHEN** a file has role `generated` or `vendor`
- **THEN** the system performs no void-mutator analysis for that file

#### Scenario: Transitive mutation not flagged
- **WHEN** a void method calls `self.list.add(x)` but does not directly assign to `self`/`this` fields
- **THEN** the system emits no void-mutator violation for that method

### Requirement: Void Mutator Violation Output

The system SHALL emit `void_mutator` violations in `human`, `json`, and `sarif` output formats, including mutator count, threshold, and per-method locations.

#### Scenario: SARIF rule ID
- **WHEN** `pretender check --format sarif` runs on a file with void-mutator violations
- **THEN** the SARIF output includes results with rule id `pretender/void-mutator`