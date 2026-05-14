# Change: Add Plugin Trust Model

## Why

Pretender plugins are loaded from arbitrary URLs (git remotes, local paths), creating a supply-chain attack surface. Any user who runs `pretender plugins add <url>` today executes unreviewed code with no integrity guarantee. This change adds three defensive layers: a lock file that pins installed plugins by revision and artifact hash, a curated registry with signature verification, and a `--frozen-plugins` flag for CI enforcement.

## What Changes

- New `pretender.plugins.lock` file — pins source URL, git rev, artifact SHA-256, and install timestamp for every installed plugin
- `pretender plugins verify` command — re-checks installed plugins against lock entries
- `pretender plugins add <url>` — warns "unverified code" and requires `--i-trust-this` for non-interactive use
- `pretender plugins add <name>` (no URL scheme) — installs from curated registry with signature verification, no `--i-trust-this` required
- `--frozen-plugins` flag on `pretender check` — refuses to start if any lock entry is missing or mismatched
- Metric plugins (TOML `command` spec) — command hash is included in lock file
- New `plugin-trust` capability spec

## Impact

- Affected specs: `plugin-trust` (new), `cli-and-config` (modified: lock file format, plugins verify command, --frozen-plugins flag)
- Affected code: plugin loader, `pretender plugins` subcommand, `pretender check` entrypoint
- **BREAKING**: `pretender plugins add <url>` now requires `--i-trust-this` flag in non-interactive environments
