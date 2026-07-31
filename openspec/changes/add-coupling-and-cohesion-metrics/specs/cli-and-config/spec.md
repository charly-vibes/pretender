## ADDED Requirements

### Requirement: Coupling Metric Thresholds

The system SHALL accept coupling metric thresholds under `[thresholds]` in `pretender.toml`: `ce_max`, `ca_max`, `cbo_max`, `lcom_hs_max` (all u32, default 0 = disabled), and `cycle_detection` (bool, default false). `lcom_hs_max` SHALL represent a percentage (0-100).

#### Scenario: Coupling thresholds configured
- **WHEN** `[thresholds] ce_max = 5` is set in `pretender.toml`
- **THEN** modules with efferent coupling > 5 are flagged as violations

#### Scenario: Zero threshold disables the metric
- **WHEN** `ce_max = 0`
- **THEN** no efferent coupling violations are reported

#### Scenario: Cycle detection enabled
- **WHEN** `cycle_detection = true` is set in `pretender.toml`
- **THEN** the import graph is analysed for cycles

#### Scenario: Generated and vendor files excluded from graph
- **WHEN** a file has role `generated` or `vendor`
- **THEN** the system excludes that file from the dependency graph and all coupling analysis

### Requirement: Coupling Metric Computation

The system SHALL compute Ce, Ca, CBO, and LCOM-HS per module from the import graph and per-method field-access analysis. A module SHALL be defined as a single source file. LCOM-HS SHALL be computed for class-based languages only (Java, C++, C#, Python classes, Rust impl blocks); for non-OOP modules, LCOM = 0 and no LCOM finding is emitted.

#### Scenario: Ce computed from imports
- **WHEN** module A imports 5 distinct external modules
- **THEN** Ce = 5 for module A

#### Scenario: Ca computed from dependents
- **WHEN** module B is imported by 3 distinct modules
- **THEN** Ca = 3 for module B

#### Scenario: CBO computed as sum
- **WHEN** Ce = 4 and Ca = 2 for module C
- **THEN** CBO = 6 for module C

#### Scenario: LCOM-HS computed from field access
- **WHEN** a class has 3 methods and 2 fields, where each method accesses exactly 1 distinct field
- **THEN** LCOM-HS is computed using the Henderson-Sellers formula

#### Scenario: LCOM-HS skipped for non-OOP module
- **WHEN** a module is a Go package or Rust module without classes
- **THEN** LCOM = 0 and no LCOM finding is emitted

### Requirement: Cycle Detection

The system SHALL detect cycles in the directed import graph and report each cycle's participant modules.

#### Scenario: Simple cycle detected
- **WHEN** modules A, B, C form A → B → C → A
- **THEN** the system reports one cycle: `[A, B, C]`

#### Scenario: Acyclic graph produces no cycles
- **WHEN** the import graph contains no cycles
- **THEN** the system reports no cycles

### Requirement: Module Coupling Report

The system SHALL include coupling findings in `CheckReport` as a `modules` field containing per-module `CouplingViolation` entries, and a `cycles` field.

#### Scenario: Coupling violation in gate mode
- **WHEN** a module has Ce > ce_max in gate mode
- **THEN** the exit code indicates a violation

#### Scenario: Cycle in gate mode
- **WHEN** a cycle is detected in gate mode
- **THEN** the exit code indicates a violation

#### Scenario: SARIF output includes coupling rule IDs
- **WHEN** `pretender check --format sarif` runs on a project with coupling violations
- **THEN** the SARIF output includes results with rule ids `pretender/efferent-coupling`, `pretender/afferent-coupling`, `pretender/coupling-between-objects`, `pretender/lcom-hs`, and `pretender/import-cycle`