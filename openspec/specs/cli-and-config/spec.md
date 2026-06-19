# CLI and Configuration

## Purpose

Defines Pretender's command-line interface, configuration file schema, output formats, role detection, and plugin manifest contracts.
## Requirements
### Requirement: Init Command

The system SHALL provide `pretender init` to create `pretender.toml`, optionally install a pre-commit hook, and optionally generate GitHub Actions configuration. The command SHALL support `--non-interactive` and `--mode <mode>`.

In the current MVP, hook installation writes a repository-wide fallback shim that runs `pretender check .` and documents that `--staged --diff-only` will replace it when diff filtering is implemented.

#### Scenario: Non-interactive creates config
- **WHEN** `pretender init --non-interactive` is run in a repository without `pretender.toml`
- **THEN** the system writes a default `pretender.toml` and exits with code 0

#### Scenario: Interactive init can install hook and workflow
- **WHEN** `pretender init` is run interactively and the user opts into hook and CI setup
- **THEN** the system writes `pretender.toml`, `.git/hooks/pre-commit`, and `.github/workflows/pretender.yml`

### Requirement: Check Command

The system SHALL provide `pretender check [paths...]` as the implemented MVP scan command for explicit file and directory inputs. The command SHALL require at least one path, SHALL recursively scan directories, and SHALL support `--format human|json|sarif`, `--output <path>`, and `--mode guidance|tiered|gate`.

For the current MVP, `--staged`, `--diff-only`, and `--diff-base <ref>` are reserved CLI flags and SHALL exit with code `2` and a `not yet implemented` message when used. Likewise, `--format junit|markdown` is reserved and SHALL exit with code `2` and a `not yet implemented` message.

Mode behavior in the current MVP is:
- `guidance`: always exits `0`
- `tiered`: always exits `0` after annotating yellow/red findings
- `gate`: exits non-zero when any file or unit threshold is violated

Every successful `pretender check` run SHALL persist the report cache used by `pretender report`, regardless of output format.

#### Scenario: Human check succeeds on clean file
- **WHEN** `pretender check path/to/file.py` runs on a file with no threshold violations
- **THEN** the command exits with code `0`

#### Scenario: Guidance mode does not fail violating file
- **WHEN** `pretender check path/to/file.py --mode guidance` runs on a file with threshold violations
- **THEN** the command exits with code `0` after reporting the violations

#### Scenario: Reserved staged flag is rejected
- **WHEN** `pretender check path/to/file.py --staged` is run
- **THEN** the command exits with code `2` and reports that the feature is not yet implemented

#### Scenario: SARIF output is emitted
- **WHEN** `pretender check path/to/file.py --format sarif` is run
- **THEN** stdout contains valid SARIF 2.1.0 JSON for the analysed findings

### Requirement: Complexity Command

The system SHALL provide `pretender complexity <path>` for the current MVP. The command SHALL accept exactly one path and SHALL report cyclomatic complexity per discovered code unit, sorted worst-first.

The current MVP does NOT implement `--top <n>` or `--threshold <n>`.

#### Scenario: Complexity reports worst units first
- **WHEN** multiple code units have different cyclomatic scores
- **THEN** `pretender complexity <path>` lists the highest scoring units before lower scoring units

### Requirement: Report Command

The system SHALL provide `pretender report` to render cached results from the last successful `pretender check`. The command SHALL support `--format human|markdown|html` and `--output <path>`.

#### Scenario: Report reads cached human check results
- **WHEN** `pretender report --format markdown` is run after a successful human-format `pretender check`
- **THEN** the command renders a Markdown report from the cached results

### Requirement: Hooks Commands

The system SHALL provide `pretender hooks install` to install a native pre-commit shim and `pretender hooks uninstall` to remove a shim previously installed by Pretender.

In the current MVP, the installed shim SHALL run `pretender check .` and include a comment that diff-only filtering is deferred to future work.

#### Scenario: Hook install writes fallback shim
- **WHEN** `pretender hooks install` is run
- **THEN** `.git/hooks/pre-commit` contains a Pretender-managed shim that executes `pretender check .`

#### Scenario: Hook uninstall removes Pretender shim
- **WHEN** `pretender hooks uninstall` is run after Pretender installed the hook
- **THEN** the Pretender-managed `.git/hooks/pre-commit` file is removed

### Requirement: CI Generate Command

The system SHALL provide `pretender ci generate github` in the current MVP. The generated workflow SHALL install Pretender, run `pretender check . --format=sarif --output=pretender.sarif`, upload SARIF, append a Markdown report on failure, and fail the job when findings are present.

Providers other than `github` remain reserved and SHALL exit with code `2` and a `not yet implemented` message.

#### Scenario: GitHub workflow includes SARIF upload
- **WHEN** `pretender ci generate github` is run
- **THEN** the generated workflow uploads Pretender SARIF through GitHub Code Scanning

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

### Requirement: Schema Versioning

The system SHALL not require a config version key. Unknown config keys SHALL be ignored for forward compatibility.

#### Scenario: Unknown key is ignored
- **WHEN** `pretender.toml` contains an unknown key
- **THEN** parsing succeeds and the unknown key has no effect

### Requirement: Output Formats

The current MVP SHALL emit `human`, `json`, and `sarif` output for `pretender check`, and `human`, `markdown`, and `html` output for `pretender report`. The configuration schema MAY parse the wider output-format enum values `junit` and `markdown` for `check`, but those `check` report formats are not yet implemented by the CLI.

#### Scenario: JSON report is emitted
- **WHEN** `pretender check path/to/file.py --format json` is run
- **THEN** stdout contains valid JSON describing file reports and unit metrics

#### Scenario: SARIF validates
- **WHEN** Pretender emits SARIF output from `pretender check`
- **THEN** the output conforms to the SARIF 2.1.0 schema

### Requirement: Plugin Manifests

The system SHALL parse data-only plugin manifests made of `.scm` query files and `plugin.toml` metadata. In the current MVP, manifest parsing is implemented as a library capability and test fixture, but plugin installation, discovery, and runtime loading are not yet implemented by the CLI.

#### Scenario: Python plugin manifest parses
- **WHEN** the built-in Python `plugin.toml` is loaded by the manifest parser
- **THEN** the manifest is parsed successfully and exposes its query and assertion metadata

### Requirement: Reserved Commands

The current MVP CLI SHALL expose the following reserved commands, which currently exit with code `2` and a `not yet implemented` message: `duplication`, `mutation`, `plugins`, and `explain`.

#### Scenario: Reserved command reports not implemented
- **WHEN** `pretender explain cyclomatic` is run
- **THEN** the command exits with code `2` and reports that the feature is not yet implemented

### Requirement: Language Support

The current MVP SHALL register built-in parsers for C, C++, Go, Java, JavaScript, Python, Ruby, Rust, and TypeScript-family source files. Files with unsupported extensions SHALL be skipped by `pretender check`, and `pretender complexity <path>` on an unsupported extension SHALL fail.

#### Scenario: Rust file is analysed
- **WHEN** `pretender check example.rs` is run
- **THEN** Pretender analyses the file with the Rust parser

#### Scenario: Unsupported complexity input fails
- **WHEN** `pretender complexity example.txt` is run
- **THEN** the command exits non-zero because no parser is registered for `.txt`

