## MODIFIED Requirements

### Requirement: Check Command

The system SHALL provide `pretender check [paths...]` as the implemented MVP scan command for explicit file and directory inputs. The command SHALL require at least one path and SHALL recursively scan directories. The command SHALL support `--format human|json`, `--output <path>`, and `--mode guidance|tiered|gate`.

For the current MVP, `--staged`, `--diff-only`, and `--diff-base <ref>` are reserved CLI flags and SHALL exit with code `2` and a `not yet implemented` message when used. Likewise, `--format sarif|junit|markdown` is reserved and SHALL exit with code `2` and a `not yet implemented` message.

Mode behavior in the current MVP is:
- `guidance`: always exits `0`
- `tiered`: exits non-zero when any unit violates a configured threshold
- `gate`: exits non-zero when any unit violates a configured threshold

#### Scenario: Human check succeeds on clean file
- **WHEN** `pretender check path/to/file.py` runs on a file with no threshold violations
- **THEN** the command exits with code `0`

#### Scenario: Guidance mode does not fail violating file
- **WHEN** `pretender check path/to/file.py --mode guidance` runs on a file with threshold violations
- **THEN** the command exits with code `0` after reporting the violations

#### Scenario: Reserved staged flag is rejected
- **WHEN** `pretender check path/to/file.py --staged` is run
- **THEN** the command exits with code `2` and reports that the feature is not yet implemented

#### Scenario: Reserved sarif output is rejected
- **WHEN** `pretender check path/to/file.py --format sarif` is run
- **THEN** the command exits with code `2` and reports that the feature is not yet implemented

### Requirement: Complexity Command

The system SHALL provide `pretender complexity <path>` for the current MVP. The command SHALL accept exactly one path and SHALL report cyclomatic complexity per discovered code unit, sorted worst-first.

The current MVP does NOT implement `--top <n>` or `--threshold <n>`.

#### Scenario: Complexity reports worst units first
- **WHEN** multiple code units have different cyclomatic scores
- **THEN** `pretender complexity <path>` lists the highest scoring units before lower scoring units

### Requirement: Configuration Schema

The system SHALL read `pretender.toml` with tables for `[pretender]`, `[thresholds]`, role-specific threshold tables such as `[thresholds.test]`, `[bands]`, `[scope]`, `[execute]`, `[plugins]`, `[output]`, and `[roles]`. The `mode` value SHALL be one of `guidance`, `tiered`, or `gate`. The implicit default role SHALL be `app`. Unknown config keys SHALL be ignored.

#### Scenario: Role-specific thresholds override app defaults
- **WHEN** a file is assigned role `test`
- **THEN** values under `[thresholds.test]` override app threshold defaults for that file

### Requirement: Role Detection

The system SHALL assign each file a role from `app`, `library`, `test`, `script`, `generated`, or `vendor` using the current MVP resolution order:
1. explicit pragma found in the first 8 lines
2. configured `[roles]` path globs, with the most specific matching glob winning
3. built-in path heuristics
4. default `app`

The current pragma scanner SHALL accept line comments beginning with `#` or `//`, and block-comment openings beginning with `/*`.

#### Scenario: Pragma wins over path glob
- **WHEN** a file declares an explicit Pretender role pragma and also matches a configured role glob
- **THEN** the pragma role is assigned

#### Scenario: Most specific glob wins
- **WHEN** a file matches both `tests/**` and `tests/manual/**`
- **THEN** the more specific glob is assigned

#### Scenario: Heuristic role is used when no pragma or glob applies
- **WHEN** a file path contains `/vendor/` and no pragma or configured glob applies
- **THEN** the file is assigned role `vendor`

#### Scenario: Block-comment pragma is recognised in MVP
- **WHEN** a file begins with `/* pretender: role = vendor */`
- **THEN** the file is assigned role `vendor`

### Requirement: Output Formats

The current MVP SHALL emit `human` and `json` output for `pretender check`. The configuration schema MAY parse the wider output-format enum values `sarif`, `junit`, and `markdown`, but those report formats are not yet implemented by the CLI.

#### Scenario: JSON report is emitted
- **WHEN** `pretender check path/to/file.py --format json` is run
- **THEN** stdout contains valid JSON describing file reports and unit metrics

### Requirement: Plugin Manifests

The system SHALL parse data-only plugin manifests made of `.scm` query files and `plugin.toml` metadata. In the current MVP, manifest parsing is implemented as a library capability and test fixture, but plugin installation, discovery, and runtime loading are not yet implemented by the CLI.

#### Scenario: Python plugin manifest parses
- **WHEN** the built-in Python `plugin.toml` is loaded by the manifest parser
- **THEN** the manifest is parsed successfully and exposes its query and assertion metadata

## ADDED Requirements

### Requirement: Reserved Commands

The current MVP CLI SHALL expose the following reserved commands, which currently exit with code `2` and a `not yet implemented` message: `init`, `report`, `duplication`, `mutation`, `hooks`, `ci generate`, `plugins`, and `explain`.

#### Scenario: Reserved command reports not implemented
- **WHEN** `pretender report` is run
- **THEN** the command exits with code `2` and reports that the feature is not yet implemented

### Requirement: Language Support

The current MVP SHALL register a Python parser for `.py` files. Files with unsupported extensions SHALL not be analysed by `pretender check`, and `pretender complexity <path>` on an unsupported extension SHALL fail.

#### Scenario: Python file is analysed
- **WHEN** `pretender check example.py` is run
- **THEN** Pretender analyses the file with the Python parser

#### Scenario: Unsupported complexity input fails
- **WHEN** `pretender complexity example.js` is run
- **THEN** the command exits non-zero because no parser is registered for `.js`

## REMOVED Requirements

### Requirement: Init Command
**Reason**: The command is present only as a reserved stub in the current MVP.
**Migration**: Track future work in a dedicated implementation change before restoring this requirement.

### Requirement: Duplication Command
**Reason**: The command is present only as a reserved stub in the current MVP.
**Migration**: Restore via a future implementation change.

### Requirement: Mutation Command
**Reason**: The command is present only as a reserved stub in the current MVP.
**Migration**: Restore via a future implementation change.

### Requirement: Report Command
**Reason**: The command is present only as a reserved stub in the current MVP.
**Migration**: Restore via a future implementation change.

### Requirement: Hooks Commands
**Reason**: The command is present only as a reserved stub in the current MVP.
**Migration**: Restore via a future implementation change.

### Requirement: CI Generate Command
**Reason**: The command is present only as a reserved stub in the current MVP.
**Migration**: Restore via a future implementation change.

### Requirement: Plugins Command
**Reason**: The command is present only as a reserved stub in the current MVP.
**Migration**: Restore via a future implementation change.

### Requirement: Explain Command
**Reason**: The command is present only as a reserved stub in the current MVP.
**Migration**: Restore via a future implementation change.

### Requirement: Schema Versioning
**Reason**: Forward-compatibility behavior for unknown keys is implemented, but the broader schema-versioning and semver policy claims are process guidance rather than implemented runtime behavior.
**Migration**: Reintroduce once version-governance behavior is formalized.
