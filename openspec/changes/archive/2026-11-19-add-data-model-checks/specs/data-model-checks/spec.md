# data-model-checks spec delta

## Purpose

Defines threshold-based structural checks evaluated over type definitions (structs, classes, records) captured in the universal code model: type field counts and field-level primitive-obsession candidates. These checks give CI the same pre-merge visibility over data-model drift that existing checks provide over functions and parameters.

Claim boundary: the field-count check evaluates structural size only — it does not measure responsibility, cohesion, or coupling, and types that legitimately carry many fields (DTOs, entities, configuration, generated code) are accommodated via role thresholds and disabling. A `primitive_field` finding is a candidate signal that a domain concept may warrant a domain type, not by itself a diagnosis of primitive obsession.

## ADDED Requirements

### Requirement: Excessive Field-Count Check

The system SHALL flag each type definition whose field count meets or exceeds the configured `excessive_fields_max` threshold as a violation with metric id `excessive_fields`, reporting the field count as `actual` and the threshold as `limit`. The default threshold SHALL be 10 (a provisional policy default, not an ecosystem precedent; provenance and calibration path are recorded in the change design); a value of `0` SHALL disable the check.

#### Scenario: Type definition at threshold is flagged

- **WHEN** a parsed file contains a type definition with 10 fields and `excessive_fields_max` is 10
- **THEN** the file report contains an `excessive_fields` violation with `actual = 10` and `limit = 10`

#### Scenario: Zero disables the check

- **WHEN** `excessive_fields_max` is `0` in the effective thresholds
- **THEN** no `excessive_fields` violations are emitted for any file

### Requirement: Field Primitive-Obsession Check

The system SHALL, when `primitive_field_check` is enabled, flag each field whose declared type is a bare primitive and whose name matches a domain-like pattern as a violation with metric id `primitive_field`, reporting `1` as both `actual` and `limit`, mirroring `primitive_param`. The check SHALL be disabled by default. A finding is a candidate signal that a domain concept may warrant a domain type, not by itself a diagnosis of primitive obsession.

#### Scenario: Primitive field with domain name is flagged

- **GIVEN** `primitive_field_check` is enabled
- **WHEN** a type definition has a field named `email` declared as `String`
- **THEN** the file report contains a `primitive_field` violation for that type definition

#### Scenario: Disabled by default

- **WHEN** `pretender.toml` does not set `primitive_field_check`
- **THEN** no `primitive_field` violations are emitted

### Requirement: Primitive-Field Domain Patterns

The `primitive_field` check SHALL use the same base pattern set and matching rules as the existing parameter-level check (`is_domain_param_name` in `primitive_obsession.rs`). The `domain_patterns` configuration key SHALL extend that base set; `domain_patterns` SHALL be a single app-level list (default empty) and SHALL NOT be role-overridable.

#### Scenario: Additional domain pattern extends the base set

- **GIVEN** `primitive_field_check` is enabled and `domain_patterns` adds a pattern no base pattern matches
- **WHEN** a type definition has a field matching only the added pattern with a bare primitive declared type
- **THEN** the file report contains a `primitive_field` violation for that field

### Requirement: Data-Model Finding Declaration

Data-model violations SHALL use the same violation shape as existing threshold checks: metric id, file, line range of the offending type definition or field, and actual/limit values. Human, JSON, and SARIF output SHALL all expose these violations through their existing violation channels without format-specific changes.

#### Scenario: Finding is consistent across output formats

- **WHEN** `pretender check` flags an `excessive_fields` violation and is run twice with `--format json` and `--format sarif`
- **THEN** both outputs contain the violation for the same file, line range, and metric id `excessive_fields`

### Requirement: Role Threshold Application

Role-specific threshold tables SHALL override data-model check thresholds per file under the existing role resolution order, including disabling a check for a role by setting its threshold to `0`.

#### Scenario: Test role disables excessive-field check

- **WHEN** `[thresholds.test]` sets `excessive_fields_max = 0` and a test-role file contains a type definition with 15 fields
- **THEN** no `excessive_fields` violation is reported for that file

### Requirement: Evaluation Scope

Data-model checks SHALL evaluate only type definitions present in the universal model for the analyzed file. Languages whose query file provides no type captures SHALL produce no type definitions and SHALL yield no data-model violations; this absence SHALL NOT be reported as an error.

#### Scenario: Language without type captures is silent

- **WHEN** a supported language adapter provides no `@type.*` captures and its file is checked
- **THEN** the check completes without data-model violations or error diagnostics related to type definitions
