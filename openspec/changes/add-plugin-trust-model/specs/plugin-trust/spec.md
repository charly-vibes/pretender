## ADDED Requirements

### Requirement: Plugin Lock File

The system SHALL maintain a `pretender.plugins.lock` file in the project root that pins every installed plugin to an immutable (source URL, git rev, artifact SHA-256, install timestamp) tuple.

Lock file format:

```toml
[[plugin]]
name            = "elixir"
kind            = "language"
runtime         = "data-only"      # optional; default "data-only"
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

The system SHALL write a new lock entry whenever a plugin is successfully installed via `pretender plugins add`. The system SHALL update an existing lock entry when a plugin is reinstalled or updated. The system SHALL also be able to generate lock entries for already-installed plugins via `pretender plugins lock-generate`.

#### Scenario: Lock file written on install

- **WHEN** `pretender plugins add elixir` completes successfully
- **THEN** `pretender.plugins.lock` contains a `[[plugin]]` entry for `elixir` with `rev`, `artifact_sha256`, and `installed_at` populated

#### Scenario: Metric plugin command hash captured

- **WHEN** a metric plugin with a `command` field is installed
- **THEN** the lock entry includes `command_sha256` derived from the resolved binary

#### Scenario: Lock entry updated on reinstall

- **WHEN** `pretender plugins add elixir` is run for an already-installed plugin with a different rev
- **THEN** the existing lock entry for `elixir` is replaced with the new rev and SHA-256

#### Scenario: Lock file generated for existing installation

- **WHEN** plugins were installed before lock-file support and `pretender plugins lock-generate` is run
- **THEN** the system hashes the installed plugin artifacts and writes corresponding lock entries

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

### Requirement: Plugin Runtime Field Reservation

Each `[[plugin]]` entry in `pretender.plugins.lock` SHALL accept an optional `runtime` field of type string. When the field is absent, the system SHALL treat the entry as `runtime = "data-only"`. The system SHALL accept and round-trip without error any `runtime` value it does not recognise, so future runtimes (e.g. `"wasm"`) can be added without breaking parsers written against this schema.

#### Scenario: Missing runtime field defaults to data-only

- **WHEN** a `[[plugin]]` entry is parsed and contains no `runtime` field
- **THEN** the in-memory representation reports `runtime = "data-only"` and the entry is treated as a data-only language or metric plugin

#### Scenario: Unknown runtime value is preserved

- **WHEN** a lock file contains `runtime = "wasm"` (a value not implemented by the current binary)
- **THEN** `pretender plugins verify` does not fail with a parse error, the unknown value is preserved on rewrite, and the plugin is reported as `SKIP <name>: unsupported runtime 'wasm'` with a non-zero exit only if `--frozen-plugins` is in effect

#### Scenario: data-only runtime executes no native code during metrics

- **WHEN** metric collection runs against a plugin with `runtime = "data-only"`
- **THEN** no process is spawned for that plugin during metric computation (verified by counting child-process spawns during a `pretender check` run)
