## ADDED Requirements

### Requirement: Boolean Cluster Threshold

The system SHALL accept a `bool_cluster_max: u32` field under `[thresholds]` in `pretender.toml`, defaulting to `0` (disabled). When enabled, functions with ≥ `bool_cluster_max` bare boolean parameters SHALL be flagged. Class/struct/record definitions with multiple bool fields SHALL NOT be flagged. Files without static type annotations (dynamically typed languages) SHALL be skipped.

#### Scenario: Boolean cluster exceeds threshold
- **WHEN** a function has 4 bare `bool` parameters and `bool_cluster_max = 3`
- **THEN** the system emits a `bool_cluster` violation for that code unit

#### Scenario: Boolean cluster within threshold
- **WHEN** a function has 2 bare `bool` parameters and `bool_cluster_max = 3`
- **THEN** the system emits no violation

#### Scenario: Zero threshold disables the check
- **WHEN** `bool_cluster_max = 0`
- **THEN** no boolean cluster analysis is performed

### Requirement: Primitive Domain Parameter Check (Optional)

The system SHALL accept a `primitive_param_check: bool` field under `[thresholds]` in `pretender.toml`, defaulting to `false` (disabled). When enabled, functions with `String`/`int` parameters whose names match known domain patterns SHALL be flagged.

#### Scenario: Primitive domain param detected
- **WHEN** `primitive_param_check = true` and a function has a parameter `email: String`
- **THEN** the system emits a `primitive_param` violation for that code unit

#### Scenario: Common primitive names excluded
- **WHEN** `primitive_param_check = true` and a function has parameters `user_id: String`, `name: string`, `path: String`
- **THEN** the system emits no `primitive_param` violation (these are legitimate primitive uses)

#### Scenario: Dynamically typed file skipped
- **WHEN** `primitive_param_check = true` and a Python/JavaScript file has no type annotations on any parameter
- **THEN** the system emits no violations and includes a note in diagnostic output

### Requirement: Violation Output

The system SHALL emit `bool_cluster` and `primitive_param` violations in `human`, `json`, and `sarif` output formats. Boolean cluster violations SHALL include the parameter count and threshold.

#### Scenario: SARIF rule IDs
- **WHEN** `pretender check --format sarif` runs on a file with boolean-cluster or primitive-param violations
- **THEN** the SARIF output includes results with rule ids `pretender/bool-cluster` and `pretender/primitive-param`

#### Scenario: Generated and vendor files excluded
- **WHEN** a file has role `generated` or `vendor`
- **THEN** the system performs no primitive-obsession analysis for that file