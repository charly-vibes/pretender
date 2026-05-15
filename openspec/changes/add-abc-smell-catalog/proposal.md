# Change: Add Versioned ABC Smell-Weight Catalog

**BREAKING CHANGE**

## Why

ABC scores depend on per-call `smell_weight` values, but these weights are currently undocumented and embedded in language adapter code. Different versions of pretender or different adapters may produce different scores for the same code, making the metric non-reproducible and non-portable. A versioned, user-inspectable weight catalog shipped with the binary solves this.

This is a breaking change because publishing the catalog pins weight values: future improvements to weights will change scores for existing codebases.

## What Changes

- **BREAKING**: `smell_weight` values for all built-in languages are now sourced from versioned catalog files; any implicit adapter-embedded weights that differ from the catalog become breaking changes
- New versioned weight catalogs are embedded in the binary, one catalog per supported language
- Catalog format: `version = 1` + `[[patterns]]` table array with `capture`, `weight`, `component`, and `rationale` fields
- Unlisted call patterns default to `weight = 1.0`
- Users may override embedded catalogs with local files at `~/.config/pretender/smell-weights/<language>.toml`; local files shadow the shipped catalog
- New subcommand: `pretender explain abc --language <lang>` prints the active weight table for that language
- Example built-in weights: `@call.eval` → weight 2.0, component C; `@call.global_state` → weight 1.5, component B

## Impact

- Affected specs: `universal-code-model` (catalog format + weight resolution), `cli-and-config` (new command + TOML override path)
- Affected code: `src/abc/`, `src/cli/explain.rs`, `src/config/`, language adapter `.scm` + `plugin.toml` files
- Dependencies: requires `update-mvp-spec-baseline` to be applied first; the `pretender explain abc --language` CLI surface also requires the reserved `explain` command to be restored
- Migration: existing ABC scores may change when catalog weights differ from previously implicit values; teams should re-baseline thresholds after upgrading
