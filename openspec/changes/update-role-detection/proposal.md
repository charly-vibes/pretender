# Change: Update Role Detection — Remove Heuristic Tier

**BREAKING CHANGE**

## Why

The current MVP role detector uses four tiers: pragma → configured glob → built-in path heuristic → default `app`. It also scans only the first 8 lines for pragmas, accepts block-comment pragma forms, and breaks glob ties by choosing the most specific match. That behavior is real, but it is more complex than necessary and mixes configured policy with implementation-defined heuristics.

This change deliberately simplifies and fully specifies role assignment so users can predict outcomes from config and source alone.

## What Changes

- **BREAKING**: role detection changes from four tiers to exactly three tiers: pragma → glob → default `app`
- **BREAKING**: built-in path heuristics are removed from role assignment
- **BREAKING**: pragma scan window expands from the first 8 lines to the first 10 lines
- **BREAKING**: block-comment forms such as `/* pretender: role=test */` are no longer valid role pragmas
- **BREAKING**: when multiple globs match, the first defined entry in config wins instead of the most specific match
- New `--explain-roles` flag on `pretender check` prints which rule (pragma/glob/default) assigned each file's role

This proposal modifies the corrected MVP baseline introduced by `update-mvp-spec-baseline`.

## Impact

- Affected specs: `cli-and-config`
- Affected code: `src/roles.rs` (or equivalent), `src/cli/check.rs`
- Dependencies: `update-mvp-spec-baseline` must be applied first so this change modifies the corrected MVP role detector
- Migration: files that currently rely on heuristic role assignment will fall back to `app`; users who want non-`app` roles for those files must add explicit path globs or pragmas to `pretender.toml`
