# Change: Update Role Detection — Remove Heuristic Tier

**BREAKING CHANGE**

## Why

The current role detection includes an unspecified "heuristics" tier between path-glob matching and the `app` default. This tier's behavior is implementation-defined, untestable, and non-reproducible: the same file can receive different roles across pretender versions. Dropping it makes role assignment deterministic and fully specified.

## What Changes

- **BREAKING**: The heuristics tier of role detection is removed
- Role detection order is now strictly: (1) file-top pragma `// pretender: role=<role>`, (2) path glob match from `[roles]` config, (3) default `app`
- No other role assignment logic exists after this change
- New `--explain-roles` flag on `pretender check` prints which rule (pragma/glob/default) assigned each file's role
- Pragma syntax: `// pretender: role=<role>` must appear within the first 10 lines of the file

## Impact

- Affected specs: `cli-and-config`
- Affected code: `src/roles.rs` (or equivalent), `src/cli/check.rs`
- Migration: files that relied on heuristic role assignment will fall back to `app`; users who want non-`app` roles for those files must add explicit path globs or pragmas to `pretender.toml`
