# Proposal: fix-cli-spec-drift

## Why

Two scenarios in the deployed `cli-and-config` spec describe pre-MVP behavior that has since shipped, contradicting the implementation and its integration tests:

1. `Reserved command reports not implemented` claims `pretender explain <metric>` exits `2` with a not-implemented message, but `explain` ships and prints the metric documentation (`test_explain_known_metric_prints_doc` pins exit `0`, output containing the metric name, "McCabe", and the default threshold `10`).
2. `Reserved staged flag is rejected` claims `pretender check --staged` exits `2`, but `--staged` ships and filters the scan to staged files (`test_check_staged_*` family pins exit `0`, staged-only file output, success on empty staging area and on a first commit with no HEAD).

The `.espectacular/cli-and-config/` contracts for both scenarios carry `DRIFT` descriptions acknowledging this; this change removes the drift at its source.

## What Changes

- MODIFIED `Check Command` requirement: `--staged` is implemented (filters the scan to files staged in git, exit `0`, with a success message when nothing is staged); `--diff-only` and `--diff-base <ref>` remain reserved. The `Reserved staged flag is rejected` scenario is replaced by shipped `--staged` behavior (scenario id kept so the contract linkage holds).
- Docs-only: no code, no rule ids, no thresholds, no SARIF-affecting behavior.

## Impact

- Affected specs: `cli-and-config` (2 MODIFIED requirements).
- Affected code: none.
- Affected contracts: `.espectacular/cli-and-config/reserved-command-reports-not-implemented.toml` and `.espectacular/cli-and-config/reserved-staged-flag-is-rejected.toml` (DRIFT pins removed).
