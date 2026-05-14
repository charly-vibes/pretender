# Change: Add Incremental Content-Addressed Cache

## Why

`pretender check --staged` must complete within 2 seconds to be viable as a pre-commit hook.
Re-parsing and re-computing metrics for files that have not changed is the primary cost driver.
A two-layer content-addressed cache (CST + metrics) eliminates redundant work and enables CI artifact reuse.

## What Changes

- New `cache` capability: two-layer content-addressed cache stored under `~/.cache/pretender/<repo-id>/`
- Cache lookup key: `(SHA-256 of file content, Pretender version, language-plugin version)`
- New `pretender cache export` / `pretender cache import` subcommands for CI artifact sharing
- New `[cache]` table in `pretender.toml` with `enabled`, `max_age_days`, `max_size_gb`, and `path` keys
- Automatic pruning of stale entries (>30 days old or total cache >1 GB by default)

## Impact

- Affected specs: `cache` (new), `cli-and-config` (new commands + config table)
- Affected code: `src/engine/`, `src/cache/` (new), `src/cli/`
- Cross-cutting: adds a new first-party dependency on content hashing; see `design.md`
