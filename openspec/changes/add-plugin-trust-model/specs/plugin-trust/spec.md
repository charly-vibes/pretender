## ADDED Requirements

### Requirement: Plugin Lock File

The system SHALL maintain a `pretender.plugins.lock` file in the project root that pins every installed plugin to an immutable (source URL, git rev, artifact SHA-256, install timestamp) tuple.

Lock file format:

```toml
[[plugin]]
name            = "elixir"
kind            = "language"
source          = "github:elixir-lang/tree-sitter-elixir"
rev             = "a3f2c8d9e1b4f7c2a5d8e3b6f9c2a5d8"
artifact_sha256 = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
installed_at    = "2026-05-13T00:00:00Z"

[[plugin]]
name            = "eslint"
kind            = "metric"
source          = "github:pretender-tools/plugin-eslint"
rev             = "b7d1e2f3a4c5d6e7f8a9b0c1d2e3f4a5"
artifact_sha256 = "6b86b273ff34fce19d6b804eff5a3f5747ada4eaa22f1d49c01e52ddb7875b4b"
command_sha256  = "ba7816bf8f01cfea414140de5dae2ec73b00361bbef0469132d0eb37f5756a9b"
installed_at    = "2026-05-13T00:00:00Z"
```

The system SHALL write a new lock entry whenever a plugin is successfully installed via `pretender plugins add`. The system SHALL update an existing lock entry when a plugin is reinstalled or updated.

#### Scenario: Lock file written on install

- **WHEN** `pretender plugins add elixir` completes successfully
- **THEN** `pretender.plugins.lock` contains a `[[plugin]]` entry for `elixir` with `rev`, `artifact_sha256`, and `installed_at` populated

#### Scenario: Metric plugin command hash captured

- **WHEN** a metric plugin with a `command` field is installed
- **THEN** the lock entry includes `command_sha256` derived from the resolved binary

#### Scenario: Lock entry updated on reinstall

- **WHEN** `pretender plugins add elixir` is run for an already-installed plugin with a different rev
- **THEN** the existing lock entry for `elixir` is replaced with the new rev and SHA-256

---

### Requirement: Plugin Verify Command

The system SHALL provide `pretender plugins verify` that re-checks every installed plugin against its lock file entry.

For each plugin the command SHALL:
1. Re-compute the artifact SHA-256 from the installed files
2. Compare against the `artifact_sha256` in the lock entry
3. Print `ok <name>` for matching plugins and `FAIL <name>: hash mismatch` for failing plugins
4. Exit with code 0 if all plugins match; exit with a non-zero code if any fail

#### Scenario: All plugins match

- **WHEN** no installed plugin has been tampered with since the lock was written
- **THEN** `pretender plugins verify` prints `ok` for each plugin and exits with code 0

#### Scenario: Tampered plugin detected

- **WHEN** an installed plugin's files have been modified since the lock was written
- **THEN** `pretender plugins verify` prints `FAIL <name>: hash mismatch` and exits non-zero

#### Scenario: Missing lock entry

- **WHEN** a plugin is installed but has no corresponding entry in `pretender.plugins.lock`
- **THEN** `pretender plugins verify` prints `FAIL <name>: not in lock file` and exits non-zero

---

### Requirement: URL Install Trust Warning

The system SHALL warn users before executing code from an arbitrary URL and SHALL require explicit acknowledgment in non-interactive environments.

When `pretender plugins add <source>` is called with a source that contains a URL scheme (`github:`, `https:`, `http:`) or a filesystem path:

- The system SHALL print: `Warning: installing from an unverified URL executes untrusted code. Review the source before continuing.`
- In a non-interactive environment (stdin is not a TTY), the system SHALL require the `--i-trust-this` flag and exit with an error if it is absent
- In an interactive environment, the system SHALL prompt "Trust and install this plugin? [y/N]" and abort on any input other than `y` or `Y`

#### Scenario: Non-interactive without flag

- **WHEN** `pretender plugins add github:example/plugin` is run with stdin not a TTY and without `--i-trust-this`
- **THEN** the system prints an error explaining `--i-trust-this` is required and exits non-zero

#### Scenario: Non-interactive with flag

- **WHEN** `pretender plugins add github:example/plugin --i-trust-this` is run with stdin not a TTY
- **THEN** the system installs the plugin after printing the trust warning

#### Scenario: Interactive confirmation accepted

- **WHEN** `pretender plugins add github:example/plugin` is run interactively and the user inputs `y`
- **THEN** the plugin is installed

#### Scenario: Interactive confirmation rejected

- **WHEN** `pretender plugins add github:example/plugin` is run interactively and the user inputs `n` (or presses Enter)
- **THEN** the system aborts without installing and exits with code 0

---

### Requirement: Curated Registry Install

The system SHALL support installing plugins by bare name from a curated registry. Registry installs do not require the `--i-trust-this` flag.

The curated registry publishes a signed index (minisign ed25519). The system SHALL verify the registry signature before resolving a plugin name to a source URL. On signature failure, the system SHALL abort and print a clear error.

#### Scenario: Registry install by name

- **WHEN** `pretender plugins add elixir` is run (no URL scheme, no path prefix)
- **THEN** the system resolves `elixir` from the curated registry index, verifies the registry signature, installs the plugin, and writes a lock entry — without requiring `--i-trust-this`

#### Scenario: Unknown registry name

- **WHEN** `pretender plugins add nonexistent-lang` is run and the name is not in the registry
- **THEN** the system prints "Plugin 'nonexistent-lang' not found in curated registry. To install from a URL, run: pretender plugins add <url> --i-trust-this" and exits non-zero

#### Scenario: Registry signature failure

- **WHEN** the registry index signature cannot be verified
- **THEN** the system aborts with an error and does not install any plugin

---

### Requirement: WASM Sandbox Placeholder

The system architecture SHALL reserve a capability slot for a future WASM sandbox layer that would execute language plugin logic in an isolated WASM runtime rather than as native code. No implementation is required in this change; this requirement documents the intended extension point.

#### Scenario: Future WASM layer extension point

- **WHEN** a future change implements the WASM sandbox
- **THEN** it SHALL be addable without breaking the lock file schema or the registry protocol defined in this change
