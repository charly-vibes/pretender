# min-assertions-rule — Delta

## ADDED Requirements

### Requirement: Rule applies only to test-identifying units

The system SHALL evaluate the `min_assertions` rule only against code units
with test identity. A unit has test identity when either:

1. its definition carries a test-registration attribute recognized for the
   language (e.g. Rust `#[test]`), or
2. its name matches a built-in test-name pattern: `^test_`, `_test$`,
   `^test[A-Z]`, `Test$`, or `^test$`.

Units without test identity SHALL be exempt from `min_assertions` regardless
of their file's role. All other rules SHALL continue to evaluate such units
per their existing thresholds. No configuration keys SHALL be added or
changed by this capability, and role detection SHALL be unaffected.

#### Scenario: Fixture unit in a test directory is exempt

- **WHEN** a scan includes a file under a `tests/` path containing a unit
  named `simple` with zero assertions and no test attribute
- **THEN** no `min_assertions` finding is produced for that unit

#### Scenario: Rust test without assertions is still flagged

- **WHEN** a `#[test]`-attributed Rust function contains no captured
  assertion macros
- **THEN** a `min_assertions` finding is produced for that function

#### Scenario: Helper unit in a test file is exempt

- **WHEN** a test file contains a helper function named `tempdir` with zero
  assertions and no test attribute
- **THEN** no `min_assertions` finding is produced for that helper

#### Scenario: Named Python test without assertions is still flagged

- **WHEN** a Python function named `test_foo` contains no assert statements
- **THEN** a `min_assertions` finding is produced for that function

#### Scenario: Other rules are unaffected on exempt units

- **WHEN** an exempt fixture unit exceeds `cyclomatic_max`
- **THEN** the `cyclomatic_max` finding is produced as before
