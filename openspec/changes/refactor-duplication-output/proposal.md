# Change: Refactor Duplication Output to Per-Clone-Pair Findings

**BREAKING CHANGE**

## Why

The current `duplication_pct_max` metric collapses all structural duplication into a single percentage, hiding actionable detail. Engineers cannot tell which pairs of code segments are duplicated, how similar they are, or where they live. Emitting one finding per clone pair exposes the full picture and enables precise per-file SARIF annotations.

Additionally, the V0 within-file-only default is no longer justified now that cross-file detection is stable — within-file-only defaults cause false negatives in the most common case.

## What Changes

- **BREAKING**: `pretender duplication` now emits one finding per clone pair instead of an aggregate percentage
- **BREAKING**: `--cross-file` is now **on by default**; the V0 within-file-only default is removed
- **BREAKING**: `duplication_pct_max` in `pretender.toml` now gates on `duplication_ratio` (duplicated_nodes / total_nodes) computed across all discovered pairs rather than a collapsed percentage
- New per-finding fields: `pair_id`, `similarity` (0–100 integer), `size_nodes`, two `locations` (file + span)
- New CLI flags: `--min-clone-size <nodes>` (default 10), `--min-similarity <0-100>` (default 90)
- `--no-cross-file` flag added to opt back into within-file-only scanning
- SARIF output: each clone pair is one `result` with `locations[0]` as primary and `relatedLocations[0]` as the paired site
- New `duplication_ratio` formula: `duplicated_nodes / total_nodes`

## Impact

- Affected specs: `cli-and-config`, `universal-code-model`
- Affected code: `src/duplication/`, `src/cli/`, `src/output/sarif.rs`
- Migration: any `duplication_pct_max` thresholds remain valid numerically; the denominator changes from a heuristic percentage to `duplication_ratio`; CI pipelines parsing JSON output must update field names
