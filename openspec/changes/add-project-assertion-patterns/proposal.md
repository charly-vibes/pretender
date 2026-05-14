# Change: Add Project-Level Assertion Pattern Configuration

## Why

Pretender detects assertions in test files via `@assert.*` tree-sitter captures defined in language plugins. Projects commonly use custom assertion helpers (e.g., `expectThat`, `verifyInvariant`, `checkEqual`) that are semantically assertions but do not match language-default patterns. Without a way to register these, test files that use only project-specific helpers will falsely fail the `min_assertions` check.

## What Changes

- New `[assertions]` table in `pretender.toml` with a `patterns` key (array of glob strings)
- `patterns` is glob-matched against `CallSite.callee` in `test`-role files
- Project patterns EXTEND language-default assertion detection (not replace)
- `patterns = []` explicitly opts out and overrides language defaults (empty = no assertion detection from config)
- Applies only to files with role `test`

## Impact

- Affected specs: `cli-and-config`
- Affected code: `src/assertions.rs` (or equivalent), `src/config/`
- No breaking changes — existing configs without `[assertions]` behave identically
