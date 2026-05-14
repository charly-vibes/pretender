# CLI and Configuration

## Purpose

Defines Pretender's command-line interface, configuration file schema, output formats, role detection, and plugin manifest contracts.

## Requirements

### Requirement: Init Command

The system SHALL provide `pretender init` to interactively create `pretender.toml`, optionally install a pre-commit hook, and optionally generate CI configuration. The command SHALL support `--non-interactive`, `--defaults`, and `--mode <mode>`.

#### Scenario: Defaults create config
- **WHEN** `pretender init --defaults` is run in a repository without `pretender.toml`
- **THEN** the system writes a default `pretender.toml` and exits with code 0

### Requirement: Check Command

The system SHALL provide `pretender check [paths]` as the fast pass/fail scan against configured thresholds. In `gate` mode, threshold violations and parse-skip diagnostics SHALL produce a non-zero exit code. In `tiered` mode, values in the configured yellow band SHALL be annotated without failing. In `guidance` mode, the command SHALL always exit 0 after reporting findings.

The command SHALL support `--staged`, `--diff-only`, `--diff-base <ref>`, `--format <fmt>`, and `--output <path>`. When `--staged` and `--diff-only` are combined, the checked set SHALL be their intersection. If the intersection is empty, the command SHALL exit 0 and emit an informational message.

Expensive checks, including cross-file duplication and mutation, SHALL be disabled in `check` unless explicitly enabled by configuration.

#### Scenario: Gate mode fails on violation
- **WHEN** `pretender check` runs in `gate` mode and a scanned file exceeds a configured `*_max` threshold
- **THEN** the command exits with a non-zero code

#### Scenario: Staged diff-only empty intersection passes
- **WHEN** `pretender check --staged --diff-only` finds no files that are both staged and changed relative to `diff_base`
- **THEN** the command exits with code 0 and prints an informational message

### Requirement: Complexity Command

The system SHALL provide `pretender complexity [paths]` to report ABC scores and component breakdowns per code unit, sorted worst-first. The command SHALL support `--top <n>` and `--threshold <n>`.

#### Scenario: Complexity reports worst units first
- **WHEN** multiple code units have ABC scores
- **THEN** `pretender complexity` lists the highest scoring units before lower scoring units

### Requirement: Duplication Command

The system SHALL provide `pretender duplication [paths]` for structural clone detection via normalized AST subtree hashing. The command SHALL hash subtrees of at least 10 nodes by default, report clone locations, size, and similarity, and initially restrict detection to within-file clones unless cross-file scanning is explicitly requested.

#### Scenario: Within-file clone reported
- **WHEN** two matching AST subtrees of at least the minimum size appear in one file
- **THEN** `pretender duplication` reports the clone locations and similarity

### Requirement: Mutation Command

The system SHALL provide `pretender mutation [paths]` as a wrapper around per-language mutation tools. The command SHALL support `--score-min <n>` and `--format human|json`.

#### Scenario: Mutation delegates to configured tool
- **WHEN** mutation execution is enabled for a supported language
- **THEN** `pretender mutation` invokes the configured language mutation tool and reports its score

### Requirement: Report Command

The system SHALL provide `pretender report` to render a human TUI or HTML report from the last check results. The command SHALL support `--format human|html`.

#### Scenario: Report reads last results
- **WHEN** `pretender report --format html` is run after a completed check
- **THEN** the system writes an HTML report derived from the last check results

### Requirement: Hooks Commands

The system SHALL provide `pretender hooks install` to install a native pre-commit shim that runs `pretender check --staged --diff-only`, and `pretender hooks uninstall` to remove hooks previously installed by Pretender.

#### Scenario: Hook install writes shim
- **WHEN** `pretender hooks install` is run
- **THEN** `.git/hooks/pre-commit` runs `pretender check --staged --diff-only`

### Requirement: CI Generate Command

The system SHALL provide `pretender ci generate <provider>` for `github`, `gitlab`, `circle`, `azure`, and `generic`. GitHub output SHALL include SARIF upload wiring for GitHub Code Scanning.

#### Scenario: GitHub workflow includes SARIF upload
- **WHEN** `pretender ci generate github` is run
- **THEN** the generated workflow uploads Pretender SARIF through GitHub Code Scanning

### Requirement: Plugins Command

The system SHALL provide `pretender plugins list|add|remove` to manage language and metric plugins under the configured Pretender plugin directory.

#### Scenario: Plugin list shows installed plugins
- **WHEN** `pretender plugins list` is run
- **THEN** installed plugin names and versions are printed

### Requirement: Explain Command

The system SHALL provide `pretender explain <metric>` to print a metric definition, threshold behavior, and citation for known metrics.

#### Scenario: Explain known metric
- **WHEN** `pretender explain cyclomatic` is run
- **THEN** the command prints the cyclomatic complexity definition and threshold citation

### Requirement: Configuration Schema

The system SHALL read `pretender.toml` with tables for `[pretender]`, `[thresholds]`, role-specific threshold tables such as `[thresholds.test]`, `[scope]`, `[execute]`, `[plugins]`, `[output]`, and `[roles]`. The `mode` value SHALL be one of `guidance`, `tiered`, or `gate`. The implicit default role SHALL be `app`.

#### Scenario: Role-specific thresholds override app defaults
- **WHEN** a file is assigned role `test`
- **THEN** values under `[thresholds.test]` override app threshold defaults for that file

### Requirement: Role Detection

The system SHALL assign each file a role from `app`, `library`, `test`, `script`, `generated`, or `vendor`. Explicit Pretender role pragmas SHALL take priority over path globs from `[roles]`; when neither applies, the role SHALL be `app`.

#### Scenario: Pragma wins over path glob
- **WHEN** a file declares an explicit Pretender role pragma and also matches a configured role glob
- **THEN** the pragma role is assigned

### Requirement: Schema Versioning

The system SHALL not require a config version key. Unknown config keys SHALL be ignored for forward compatibility. Breaking changes to config key semantics SHALL require a major semver version and changelog entry.

#### Scenario: Unknown key is ignored
- **WHEN** `pretender.toml` contains an unknown key
- **THEN** parsing succeeds and the unknown key has no effect

### Requirement: Output Formats

The system SHALL support output formats `human`, `json`, `sarif`, `junit`, and `markdown` where applicable. SARIF output SHALL target SARIF 2.1.0 compatibility for GitHub Code Scanning and SARIF-aware IDEs.

#### Scenario: SARIF validates
- **WHEN** Pretender emits SARIF output
- **THEN** the output conforms to the SARIF 2.1.0 schema

### Requirement: Plugin Manifests

Language plugins SHALL be data-only plugin packages made of `.scm` query files and `plugin.toml` manifests; they SHALL NOT execute native code during metric collection. Metric plugins SHALL be external command-wrapper packages that declare a command, applicable languages, parser, and output mapping. Future native or WASM plugin execution models require a separate compatibility and trust specification before implementation.

#### Scenario: Language plugin declares query
- **WHEN** a language plugin is installed
- **THEN** its manifest identifies the tree-sitter grammar source and query file used to populate the universal model
