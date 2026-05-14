## ADDED Requirements

### Requirement: Filesystem Watcher

The system SHALL watch one or more paths for file-write and file-rename events and trigger a single-file re-check on each event. Paths default to the project root when not specified. Events SHALL be debounced (default: 50ms) to avoid duplicate re-checks on rapid saves.

The watcher SHALL filter events to source file extensions known to installed language plugins; changes to non-source files SHALL be silently ignored.

#### Scenario: Source file save triggers re-check

- **WHEN** `pretender watch` is running and a tracked source file is saved
- **THEN** a re-check of that file is triggered within the debounce window

#### Scenario: Non-source file ignored

- **WHEN** `pretender watch` is running and a `.json` config file is saved (with no language plugin for JSON)
- **THEN** no re-check is triggered

#### Scenario: Rapid saves debounced

- **WHEN** a file is saved three times within 30ms
- **THEN** exactly one re-check is triggered (debounced)

---

### Requirement: Single-File Re-Check Performance

With a warm incremental cache (from `add-incremental-cache`), a single-file re-check SHALL complete in less than 100ms. This requirement gates the watch mode on the incremental cache being available.

#### Scenario: Sub-100ms re-check with warm cache

- **WHEN** `pretender watch` is running with a warm cache and a single file is saved
- **THEN** the re-check completes and results are written to the SARIF output path in less than 100ms

---

### Requirement: Console Feedback

The system SHALL print a one-line status to stdout for each re-check:
- On findings: `~ <file> changed -> <N> finding(s) (<rule> <actual> > <threshold>) (<elapsed>ms)`
- On clean: `~ <file> changed -> clean (<elapsed>ms)`

#### Scenario: Console output with finding

- **WHEN** a re-check of `src/router.rs` produces one `PRT-CYCLO` finding (actual 17, threshold 10)
- **THEN** the console prints: `~ src/router.rs changed -> 1 finding (PRT-CYCLO 17 > 10)`

#### Scenario: Console output clean

- **WHEN** a re-check of `src/router.rs` produces no findings
- **THEN** the console prints: `~ src/router.rs changed -> clean`

---

### Requirement: SARIF Output Path

The system SHALL write SARIF results to a configurable output path after each re-check. The default path is `pretender.sarif` in the project root. The `--output <path>` CLI flag and the `[watch] sarif_path` config key SHALL override the default.

The SARIF file SHALL be fully rewritten (not appended) on each re-check so that SARIF-aware IDE extensions see a consistent file.

#### Scenario: Default SARIF path used

- **WHEN** `pretender watch` is started without `--output`
- **THEN** re-check results are written to `pretender.sarif` in the project root

#### Scenario: Custom SARIF path via flag

- **WHEN** `pretender watch --output /tmp/results.sarif` is started
- **THEN** re-check results are written to `/tmp/results.sarif`

#### Scenario: SARIF fully rewritten on each re-check

- **WHEN** a file with 2 findings is saved, then fixed and saved again
- **THEN** after the second save, the SARIF file contains 0 findings (not the previous 2)

---

### Requirement: JSON-RPC Push Socket

The system SHALL support an optional `--port <n>` flag that opens a TCP JSON-RPC push socket. After each re-check the system SHALL push a `pretender/findings` notification to all connected clients with the SARIF result payload. Clients that disconnect SHALL be silently removed. Multiple simultaneous clients SHALL be supported.

#### Scenario: Findings pushed to connected client

- **WHEN** `pretender watch --port 7777` is running and a source file is saved producing a finding
- **THEN** a `pretender/findings` JSON-RPC notification is pushed to all connected clients

#### Scenario: Client disconnect is silent

- **WHEN** a client disconnects while `pretender watch --port 7777` is running
- **THEN** the watcher continues operating without error

---

### Requirement: Clean Shutdown

The system SHALL exit with code 0 on SIGINT or SIGTERM, printing `Stopping pretender watch.` before exiting. The system SHALL flush the final SARIF output before exiting.

#### Scenario: SIGINT causes clean exit

- **WHEN** `pretender watch` receives SIGINT (Ctrl-C)
- **THEN** the process prints "Stopping pretender watch.", writes any pending SARIF, and exits with code 0

#### Scenario: SIGTERM causes clean exit

- **WHEN** `pretender watch` receives SIGTERM
- **THEN** the process exits with code 0
