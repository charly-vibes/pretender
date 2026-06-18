# Change: Add Doctor Command

## Why

Users setting up Pretender in a new repo — or debugging a broken CI run — have no single command to tell them what's wrong. They have to guess whether `pretender.toml` is missing, malformed, the hook isn't installed, or a plugin manifest path is wrong. `pretender doctor` fixes this with a single self-diagnostic command that checks every precondition and prints a clear pass/fail verdict for each.

## What Changes

- New `pretender doctor` command — runs a series of health checks and prints a per-check result
- Checks performed (in order):
  1. **Git context** — working directory is inside a git repository
  2. **Config present** — `pretender.toml` exists in the working directory
  3. **Config valid** — `pretender.toml` is valid TOML and passes schema validation
  4. **Hook installed** — a Pretender-managed pre-commit hook exists at `.git/hooks/pre-commit`
  5. **Hook executable** — the hook file has the executable bit set
  6. **Plugin manifests** — any `.toml` files in the external metrics directory (`$PRETENDER_METRICS_DIR` or `~/.config/pretender/metrics/`) are well-formed TOML; trivially passes when the directory is absent or empty
- Exit code `0` when all non-skipped checks pass; exit code `1` when one or more checks fail; skipped checks do not affect the exit code
- Human output by default (`--format human`): one line per check with a `✓` / `✗` / `⚠` prefix and a short explanation, followed by a summary `X/6 checks passed`
- `--format json` emits a JSON array of `{ name, status, message }` objects for scripting and CI integration
- No `--output` flag — output is always written to stdout

## Impact

- Affected specs: `cli-and-config` (adds Doctor Command requirement)
- Affected code: new `doctor` subcommand in `pretender/src/main.rs`, new `doctor.rs` module
- No breaking changes
