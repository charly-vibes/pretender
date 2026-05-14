## 1. Filesystem Watcher

- [ ] 1.1 Add `notify` (or equivalent) crate for cross-platform filesystem events
- [ ] 1.2 Watch specified paths (default: project root) for write/rename events
- [ ] 1.3 Debounce rapid saves (default: 50ms) to avoid duplicate re-checks
- [ ] 1.4 Filter events to source file extensions known to installed language plugins

## 2. Single-File Re-Check

- [ ] 2.1 On file save event: re-parse and re-run metrics for the changed file only
- [ ] 2.2 Read warm cache for unchanged dependencies (requires `add-incremental-cache`)
- [ ] 2.3 Ensure single-file re-check completes in < 100ms with warm cache
- [ ] 2.4 Emit findings to SARIF output path (default: `pretender.sarif`, overwritable)

## 3. Console Output

- [ ] 3.1 Print `~ <file> changed -> <N> finding(s) (<rule> <actual> > <threshold>)` on findings
- [ ] 3.2 Print `~ <file> changed -> clean` when no findings
- [ ] 3.3 Show elapsed time per re-check (e.g., `(12ms)`)

## 4. JSON-RPC Push Socket (optional)

- [ ] 4.1 Add `--port <n>` flag to `pretender watch`
- [ ] 4.2 On each re-check: push a JSON-RPC notification to connected clients with SARIF result payload
- [ ] 4.3 Accept multiple simultaneous clients; disconnect gracefully on client drop

## 5. Signal Handling

- [ ] 5.1 Catch SIGINT and SIGTERM; flush SARIF output and exit with code 0
- [ ] 5.2 Print `Stopping pretender watch.` on clean exit

## 6. Configuration

- [ ] 6.1 Read `[watch]` section from `pretender.toml` for `sarif_path` and `debounce_ms`
- [ ] 6.2 CLI flags (`--output`, `--port`) override config file values

## 7. Tests

- [ ] 7.1 Integration test: save a file with a cyclomatic violation; SARIF output contains the finding
- [ ] 7.2 Integration test: fix the violation; SARIF output is clean after next save
- [ ] 7.3 Performance test: single-file re-check with warm cache completes in < 100ms
- [ ] 7.4 Test: SIGINT causes clean exit with exit code 0
