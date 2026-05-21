---
date: "2026-05-21"
project: "pretender-mvp"
sessions_analyzed: 6
type: reflection
---

## Project-Specific AI Context
_Last reflected: 2026-05-21 · 7 sessions analyzed_

### Conventions

- **TDD pipeline order**: plan → red → green → ro5u review → verify-close. Every issue follows this via `wai pipeline current`. Don't skip ro5u — it has caught correctness bugs (SARIF file violations, assertion counting scoping, branch mapping) in multiple sessions.
- **New language plugin layout**: `languages/{lang}/plugin.toml` + `metrics.scm` + `pretender/src/{lang}.rs` mirroring `javascript.rs`. Wire extension dispatch in `main.rs`. Add tree-sitter crate to `Cargo.toml`.
- **Test fixtures must be staged into temp dirs**: placing a fixture under `tests/` matches `**/tests/**` glob → role detected as `test` with stricter thresholds (cyclomatic_max=3), causing unexpected failures. Use the `stage_fixture` helper to copy into a per-test `TempDir` so the default `app` role applies.
- **Report cache path**: `pretender report` reads from `.pretender/last-check.json`. The `check` command must write this cache on every successful run for `report` to work.
- **Operating modes**: `guidance` and `tiered` exit 0; `gate` exits non-zero on threshold violations. The current default before `pretender-6aw` was gate-style unconditionally.

### Common Gotchas

- **`.tsx` files require `LANGUAGE_TSX`**, not `LANGUAGE_TYPESCRIPT`. Using the wrong parser silently fails on JSX syntax.
- **`@branch.switch_case` is silently ignored by the engine**. The engine's `branch_mapping` only recognizes `@branch.if`. Use `@branch.if` for all branch captures including switch cases (each case counts as +1 cyclomatic).
- **Rust grammar node type**: `compound_assignment_expr`, not `compound_assignment_expression`. Verified from `grammar.json`.
- **SARIF file violations need explicit loop**: the `push_result` pattern naturally only iterates unit violations. File-level violations must be added in a separate loop with `startLine=1` (SARIF convention for file-scope findings).
- **Assertion counting must skip nested function definitions**: counting `@assert.*` captures recursively includes nested helper functions, letting outer test functions satisfy `min_assertions` accidentally. `count_captured_nodes` skips nested definitions.
- **Built-in parser manifest caching**: use `OnceLock<Result<Manifest>>` not `OnceLock<Manifest>`. The `expect()` pattern panics on invalid manifests; propagate the error instead.
- **`bd close` "auto-export: git add failed"**: expected when no Dolt remote is configured. Issue closure persists to local Dolt db; the warning is only the git-export step. Not a real failure.
- **OpenSpec `MODIFIED` sections don't validate semantically**: `openspec validate --all --strict` passes even when a `MODIFIED` block introduces requirements that don't exist in the current baseline. Proposals were historically written against an aspirational baseline — verify against implemented behavior, not spec aspirations.

### Steps That Tend to Require Multiple Tries

- **Pipeline run state files go stale**: if `.wai/pipeline-runs/*.yml` shows `current_step: 5/5` from a prior topic, the pipeline won't advance. Requires manual edit to reset to the correct step.
- **`cargo clippy --all-targets -- -D warnings`**: dead code in unused modules (e.g., manifest helpers in `python.rs` not yet wired) fails under `-D warnings`. Don't leave unconnected public functions in source modules.

### Architecture Notes

- **QueryEngine is the central abstraction**: `PythonParser` delegates to `QueryEngine` with `queries/python.scm`. All language parsers should route through `QueryEngine`, not write independent CST traversal logic.
- **Branch weight injection**: `QueryEngine::new_with_branch_weights` threads plugin-defined cognitive weights into `Branch` values. Plugin manifests are cached with `OnceLock` to avoid re-parsing per file.
- **Smell weights for calls**: `plugin.toml` `[smell_weights]` keys are exact callee names (e.g., `eval=5`). They are threaded as `call_weights: &BTreeMap<String, f64>` through `build_block → walk_block → collect_nested_blocks → visit_child`. Default is `1.0` for unmatched callees.
- **Role detection priority**: explicit leading-comment pragma > configured `[roles]` globs > built-in heuristics (vendor/generated/test/library/script path patterns) > `app` fallback. Glob conflicts resolved by non-wildcard character count (more specific wins).
- **`Language::Rust`** already existed in `model.rs` before the Rust plugin was implemented — check the model before adding enum variants.
- **OpenSpec `update-mvp-spec-baseline`** is a prerequisite for most active feature proposals (SARIF, watch mode, explain, plugins, duplication, assertion enforcement, suppression syntax, ABC smell catalogs, role detection). Implementing dependent proposals without applying this baseline first causes spec drift.