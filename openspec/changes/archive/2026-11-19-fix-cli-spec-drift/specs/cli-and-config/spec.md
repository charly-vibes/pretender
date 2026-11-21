## MODIFIED Requirements

### Requirement: Check Command

The system SHALL provide `pretender check [paths...]` as the implemented MVP scan command for explicit file and directory inputs. The command SHALL require at least one path, SHALL recursively scan directories, and SHALL support `--format human|json|sarif`, `--output <path>`, and `--mode guidance|tiered|gate`.

`--staged` SHALL filter the scan to files staged in git, diffing the git index against the repository's HEAD (or against the empty tree on a repository with no prior commit, so staged files are still detected; untracked unstaged files are never scanned) and SHALL exit `0` with a success message when the staging area is empty. `--diff-only` and `--diff-base <ref>` remain reserved CLI flags and SHALL exit with code `2` and a `not yet implemented` message when used.

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
- **WHEN** `pretender check . --staged` is run in a repository with one staged file and one unstaged file
- **THEN** the command exits with code `0` and reports only the staged file

#### Scenario: SARIF output is emitted
- **WHEN** `pretender check path/to/file.py --format sarif` is run
- **THEN** stdout contains valid SARIF 2.1.0 JSON for the analysed findings

### Requirement: Reserved Commands

The current MVP CLI SHALL expose `plugins` as the reserved command, which exits with code `2` and a `not yet implemented` message. The `duplication <file>`, `mutation`, and `explain <metric>` commands are implemented: `duplication` and `explain` exit `0`, and `mutation` exits `0` or `1`.

#### Scenario: Reserved command reports not implemented
- **WHEN** `pretender plugins list` is run
- **THEN** the command exits with code `2` and reports that the feature is not yet implemented
