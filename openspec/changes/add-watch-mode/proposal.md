# Change: Add Watch Mode

## Why

Pretender provides fast feedback in CI and at commit time, but developers writing code today have no way to see quality findings as they save files — short of running the pre-commit hook manually. Writing a full LSP server is out of scope, but a filesystem watcher that re-checks changed files on save and writes SARIF covers 90% of the use case: IDE squiggles update within seconds of saving, without a build step or language server handshake.

## What Changes

- New `pretender watch [paths]` command — starts a filesystem watcher on specified paths (default: project root)
- On file save: re-checks the changed file and writes SARIF to a configured output path
- Console feedback: `~ src/router.rs changed -> 1 finding (PRT-CYCLO 17 > 10)` or `~ src/router.rs changed -> clean`
- Configurable SARIF output path (default: `pretender.sarif`)
- Optional `--port <n>` for a JSON-RPC push socket for editors that prefer push over polling
- With warm cache (requires `add-incremental-cache`), single-file re-check SHALL complete in < 100ms
- Exits cleanly on SIGINT/SIGTERM

## Impact

- Affected specs: `watch` (new), `cli-and-config` (adds watch command)
- Affected code: new `watch` subcommand, filesystem watcher integration, optional JSON-RPC push socket
- Dependencies: `add-incremental-cache` (warm cache is required for < 100ms single-file re-check)
- No breaking changes
