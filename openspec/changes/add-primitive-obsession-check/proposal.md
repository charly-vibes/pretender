# Change: Add primitive-obsession check

## Why

Primitive obsession (domain concepts as raw strings/numbers) and boolean blindness (bare bool params with invisible meaning) are the most common type-system integrity violations. Three or more bare `bool` parameters in a single function signature is a reliable signal that domain state is being conflated. A deterministic flag lets teams gate against these patterns at check-in time.

## What Changes

- Add two per-function heuristics:
  1. **Boolean cluster detection**: flag functions with ≥ N bare `bool` parameters (threshold: `bool_cluster_max: u32` under `[thresholds]`, default `0`).
  2. **Primitive domain param detection** (optional / future): flag `String`/`int` parameters whose name suggests a domain type (contains `email`, `url`, `id`, `name`, `path`, `date`) — default disabled.
- Boolean cluster detection works for languages with static type annotations (Rust, Java, TypeScript, C++, Go, C#, Julia). For dynamically typed languages (Python, Ruby, JavaScript) without type annotations, the check is skipped — the limitation is documented in the output.
- Files with role `generated` or `vendor` are excluded from this check.
- Configuration-struct/flag-object types with multiple bool fields are excluded (struct/class definitions are not flagged — only function parameter lists).
- Findings attach per `CodeUnit` in `UnitReport.violations` with metric name `bool_cluster` or `primitive_param`.
- Gatable in `gate` mode.

## Impact

- Affected specs: `cli-and-config` (ADDED boolean-cluster and primitive-param threshold fields, violation metric names).
- Affected code: `pretender/src/config.rs` (new `bool_cluster_max`, `primitive_param_check: bool`), `pretender/src/main.rs` (evaluation in per-unit metric pass, violation creation, exit-code extension).
- Minimal — no new modules; the per-language adapters already capture parameter types. For current adapters that don't capture types, add `@type` captures to bool/string matching queries.
- User-extensible domain-name pattern list via `[patterns.primitive_params] extra = ["token", "secret"]` in config.
- Non-breaking: both fields default to disabled (0 / false).

## Dependencies

- Requires `@param.type` AST captures across all language adapters — foundational work shared with other type-aware checks.