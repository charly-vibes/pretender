# Change: Add Suppression Syntax

## Why

Repo-wide rule disables in `pretender.toml` are too coarse: teams need to suppress a specific
violation at a specific site (e.g., an intentionally complex parser function) without silently
disabling the rule everywhere. Inline pragma suppressions with mandatory reasons and optional
expiry dates provide a targeted, auditable escape hatch.

## What Changes

- Inline comment pragma: `// pretender: allow[rule1, rule2] reason="..." until="YYYY-MM-DD"`
- Reason is mandatory; empty reason is a parse-time validation error
- `allow[*]` (wildcard) is forbidden; rules must be enumerated
- Scope: the pragma suppresses the next `CodeUnit` (or the entire `Module` if placed at file top)
- Expiry: after `until=YYYY-MM-DD`, the suppression is ignored and the violation resurfaces
- New `pretender suppressions list` command — reports every active suppression across the repo
- Language plugins declare Pretender pragma comment syntax in `plugin.toml` via a shared `[pragmas]` key

## Impact

- Affected specs: `suppressions` (new), `cli-and-config` (new command + plugin spec extension)
- Affected code: `src/suppressions/` (new), `src/engine/`, `src/cli/`
- No `design.md` needed: the suppression model is self-contained and not cross-cutting
