## ADDED Requirements

### Requirement: Watch Command

The system SHALL provide a `pretender watch [paths]` command that starts a filesystem watcher on the specified paths (default: project root) and re-checks changed source files on save.

Flags:
- `[paths]` — one or more paths to watch (default: project root)
- `--output <path>` — SARIF output path (default: `pretender.sarif`)
- `--port <n>` — optional JSON-RPC push socket port

The command SHALL run until interrupted by SIGINT or SIGTERM.

#### Scenario: Watch starts on project root by default

- **WHEN** `pretender watch` is run from a project directory without path arguments
- **THEN** the watcher monitors the project root recursively

#### Scenario: Watch on explicit paths

- **WHEN** `pretender watch src/ lib/` is run
- **THEN** only files under `src/` and `lib/` trigger re-checks

#### Scenario: Watch with custom output path

- **WHEN** `pretender watch --output build/pretender.sarif` is run
- **THEN** SARIF results are written to `build/pretender.sarif` after each re-check

#### Scenario: Watch with JSON-RPC socket

- **WHEN** `pretender watch --port 7777` is run
- **THEN** a JSON-RPC push socket is available at `localhost:7777`

---

### Requirement: Watch Configuration in pretender.toml

The system SHALL support a `[watch]` section in `pretender.toml` for persistent watch configuration. CLI flags SHALL override config values.

```toml
[watch]
sarif_path   = "pretender.sarif"   # default SARIF output path
debounce_ms  = 50                  # event debounce window in milliseconds
```

#### Scenario: Config file sets SARIF path

- **WHEN** `pretender.toml` contains `[watch]\nsarif_path = "out/watch.sarif"` and `pretender watch` is run without `--output`
- **THEN** SARIF results are written to `out/watch.sarif`

#### Scenario: CLI flag overrides config file

- **WHEN** `pretender.toml` sets `sarif_path = "out/watch.sarif"` and `pretender watch --output /tmp/a.sarif` is run
- **THEN** SARIF results are written to `/tmp/a.sarif`
