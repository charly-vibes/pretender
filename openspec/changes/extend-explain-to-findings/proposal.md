# Change: Extend explain to Findings

## Why

`pretender explain <metric>` currently prints a metric definition and threshold citation. When a developer receives a CI failure like `PRT-CYCLO 17 > 10 in src/router.rs::handle_request`, they must manually locate the function and mentally reconstruct why it scored 17. This gap breaks the CI-to-editor action loop. Extending `explain` to accept a finding id or `file::function` path prints the actual score, top contributing nodes, and concrete remediation suggestions — bridging the failure message to developer action without requiring a second tool.

## What Changes

- `pretender explain <file>::<function>` — explains the most recent finding for the named function
- `pretender explain <finding-id>` — explains a specific finding by its SARIF result id
- Output gains three new sections:
  1. "This finding" — file path+span, function name, actual score, zone (green/yellow/red)
  2. "Top contributors" — ranked list of line + node description + contribution value
  3. "What helps" — 2-3 mechanical suggestions + suppression pragma template
- Data source: cached metric results from the last `pretender check` run (`add-incremental-cache` dependency)
- When no cache entry exists: error with suggestion to run `pretender check` first

## Impact

- Affected specs: `cli-and-config` (explain command extended)
- Affected code: `pretender explain` command handler, cache reader
- Dependencies: `update-mvp-spec-baseline` must be applied first; the reserved `explain` command must be restored before this extension; `add-incremental-cache` must be applied first for cache data to exist
- Cache scope: this change extends the most-recent check cache with an explainable findings index; the base cache capability does not provide that index by itself
- No breaking changes — existing `pretender explain <metric>` behavior is preserved
