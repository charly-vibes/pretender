# Change: Reconcile OpenSpec Baseline with Implemented MVP

## Why

The published OpenSpec baseline currently describes a much larger Pretender surface area than the repository actually implements. This creates false completion signals, makes planning harder, and causes follow-on proposals to diff against an inaccurate baseline.

## What Changes

- Narrow the CLI baseline to the implemented MVP: `check` and `complexity`
- Record the current behavior of reserved subcommands and flags that exit with "not yet implemented"
- Align mode semantics, output formats, and role detection with current code
- Align the universal-code-model spec with the current parser/engine behavior, including Python-only support and parse-error handling
- Remove or defer unsupported plugin/runtime/versioning claims from the baseline specs

## Impact

- Affected specs: `cli-and-config`, `universal-code-model`
- Affected code: none
- Follow-up: active proposals should be rebased against this corrected baseline before implementation
