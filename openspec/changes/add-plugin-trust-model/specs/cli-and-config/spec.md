## ADDED Requirements

### Requirement: Plugins Verify Subcommand

The system SHALL expose `pretender plugins verify` as a subcommand that re-checks all installed plugins against the lock file and exits non-zero if any mismatch is found.

#### Scenario: Verify clean installation

- **WHEN** `pretender plugins verify` is run and all plugins match the lock
- **THEN** each plugin prints `ok <name>` and the command exits with code 0

#### Scenario: Verify detects mismatch

- **WHEN** an installed plugin artifact does not match the lock entry SHA-256
- **THEN** `pretender plugins verify` prints `FAIL <name>: hash mismatch` and exits non-zero

---

### Requirement: Frozen Plugins Flag

The system SHALL support a `--frozen-plugins` flag on `pretender check` that verifies all installed plugins match the lock file before running any analysis. If any plugin is missing from the lock or its hash does not match, the command SHALL exit with a non-zero code and print an actionable error message.

#### Scenario: Frozen plugins passes with matching lock

- **WHEN** `pretender check --frozen-plugins` is run and all plugins match the lock file
- **THEN** analysis proceeds normally

#### Scenario: Frozen plugins fails on mismatch

- **WHEN** `pretender check --frozen-plugins` is run and a plugin's hash does not match the lock
- **THEN** the command exits non-zero before running analysis and prints: "Plugin lock mismatch: <name>. Run 'pretender plugins verify' to diagnose."

#### Scenario: Frozen plugins fails on missing lock file

- **WHEN** `pretender check --frozen-plugins` is run and `pretender.plugins.lock` does not exist
- **THEN** the command exits non-zero and prints: "No lock file found. Run 'pretender plugins add' to generate one."

---

### Requirement: Plugin Lock File Config Format

The `pretender.plugins.lock` TOML file SHALL use an array-of-tables structure with one `[[plugin]]` entry per installed plugin. Each entry SHALL contain: `name`, `kind`, `source`, `rev`, `artifact_sha256`, `installed_at`. Metric plugins with a `command` field SHALL additionally include `command_sha256`.

#### Scenario: Lock file round-trip

- **WHEN** a lock file is written by `pretender plugins add` and then read back
- **THEN** all fields are preserved with identical values and types

#### Scenario: Metric plugin lock entry includes command hash

- **WHEN** a metric plugin specifying `command = "eslint --format json {files}"` is installed
- **THEN** the lock entry contains `command_sha256` for the resolved `eslint` binary
