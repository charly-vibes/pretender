# Tasks: add-data-model-checks

Each task names the spec scenario it satisfies and follows red→green→refactor (test first, then implementation, tidy separately).

## 1. Universal model: TypeDefinition entity

- [ ] 1.1 RED: write failing unit tests in `model.rs` for `TypeDefinition { name, kind, span, fields, language }` with `Field { name, type_name: Option<String>, span }` and `Module.type_definitions` defaulting to empty — covers "Struct maps to type definition" and "Members are not duplicated" (fields on TypeDefinition, methods remain CodeUnits). GREEN: add the types + serde derives + `Default` impl. Verify: `cargo test model`.
- [ ] 1.2 REFACTOR: confirm no existing consumer constructs `Module` literally in a way that breaks (add `..Default::default()` where needed); verify `cargo build` and existing tests still pass.

## 2. Query engine: optional @type.* captures

- [ ] 2.1 RED: write failing engine test exercising `@type.definition`/`@type.name`/`@type.field`/`@type.field.name`/`@type.field.type` captures against a small inline Rust fixture (struct + anonymous inner type + method) asserting one TypeDefinition with only struct-level fields — covers "Type captures populate type definition". GREEN: implement capture family + innermost-enclosing-type binding in `engine.rs`. Verify: `cargo test engine`.
- [ ] 2.2 RED: write failing/clarifying engine test asserting a language query without type captures parses to an empty `type_definitions` list with no error diagnostic — covers "Adapter without type captures stays valid". GREEN if already passing after 2.1; else fix engine. Verify: `cargo test engine`.

## 3. ViolationReport.line (additive)

- [ ] 3.1 RED: write failing test that `ViolationReport` serializes without `line` when `None` (byte-compatible with today's JSON) and emits `line` when `Some` — supports "Finding is consistent across output formats". GREEN: add `line: Option<u32>` with `#[serde(default)]` and thread it into the SARIF region (`start_line`) when present. Verify: `cargo test` (SARIF + JSON emission tests).

## 4. data_model checks (pure module)

- [ ] 4.1 RED: write failing unit tests for `excessive_fields` in a new `data_model.rs`: 10 fields at threshold 10 flags one violation with `actual = 10`; threshold 0 yields none — covers "Type definition at threshold is flagged" and "Zero disables the check". GREEN: implement the check as a pure function over `&[TypeDefinition]`. Verify: `cargo test data_model`.
- [ ] 4.2 RED: write failing unit tests for `primitive_field`: enabled + `email: String` flags `primitive_field`; disabled by default yields none; extra domain patterns respected — covers "Primitive field with domain name is flagged" and "Disabled by default". GREEN: implement alongside 4.1's function. Verify: `cargo test data_model`.
- [ ] 4.3 REFACTOR: extract shared "bare primitive type" predicate with `primitive_obsession.rs` if identical logic emerges; keep both call sites. Verify: `cargo test data_model primitive_obsession`.

## 5. Config keys

- [ ] 5.1 RED: write failing config tests for `AppThresholds.excessive_fields_max` (default 10) and `AppThresholds.primitive_field_check` (default false), including role-table override (`[thresholds.test]` setting `excessive_fields_max = 0` wins) — covers "Test role disables excessive-field check" at the merge level. GREEN: extend `config.rs` with serde defaults. Verify: `cargo test config`.
- [ ] 5.2 RED: write failing config test for `AppThresholds.domain_patterns` (default empty list) whose value reaches the `data_model` analysis input — covers "support additional domain-name patterns via `domain_patterns`". Also assert `domain_patterns` is a single app-level list that role tables do not merge (spec: not role-overridable). GREEN: extend `config.rs`. Verify: `cargo test config`.

## 6. Language adapters: type captures

Fixtures for generic-capable languages (Rust, Go, Java, C#, TypeScript) include one generic type case so captured `name`/`type_name` values stay free of grammar syntax. These language batches (6.1–6.6) land as separable PRs to respect the ~400-line PR cap; a language failing its fixture blocks only its own captures.

- [ ] 6.1 Add `@type.*` captures to `rust` and `go` `metrics.scm` (structs; record-kind mapping n/a) with a per-language fixture round-trip test (file parses, expected TypeDefinition count/fields). Verify: per-language round-trip tests.
- [ ] 6.2 Add captures to `python` (class_definition; `@dataclass` ships as `Class` per design D3) and `ruby` (class_definition; attr_* as fields if the grammar exposes them cleanly — otherwise ship Ruby with zero captures per design D2) with fixtures. Verify: per-language round-trip tests.
- [ ] 6.3 Add captures to `java` (class + record → Class/Record) and `csharp` (class + record) with fixtures. Verify: per-language round-trip tests.
- [ ] 6.4 Add captures to `javascript` and `typescript` (class declarations, TS property signatures on classes) with fixtures. Verify: per-language round-trip tests.
- [ ] 6.5 Add captures to `c` and `cpp` (structs only — unions are not captured; C++ class → Class) with fixtures. Verify: per-language round-trip tests.
- [ ] 6.6 Add captures to `clojure` (`defrecord` → Record, `deftype` → Class where the grammar exposes distinct nodes), `julia` (`struct`/`mutable struct` → Struct), and `r` (likely no natural type node — ship zero captures per design D2) with fixtures where applicable. Verify: per-language round-trip tests, or a documented zero-capture case for R.
- [ ] 6.7 Record the final coverage matrix (language → captured kinds → type nodes → fixture path) in this change's `design.md` appendix table; verify each listed language's test passes.

## 7. Check wiring + CLI integration

- [ ] 7.1 RED: write failing integration fixture test (new fixture file with a 10-field struct and an `email: String` field, plus a fixture `pretender.toml` that enables `primitive_field_check`, which is off by default) that `pretender check <fixture> --format json` emits `excessive_fields` and `primitive_field` violations with correct `actual`/`limit` and `line` — covers "Finding is consistent across output formats" (JSON leg; SARIF leg in 7.3) and "Language without type captures is silent" (same fixture set plus a zero-capture language, if any adapter lags). GREEN: wire `data_model::analyze` into `main.rs` beside the other app-threshold checks, pushing `ViolationReport`s with spans. Verify: `cargo test cli_test`.
- [ ] 7.2 RED: extend the integration test with a `pretender.toml` using `[thresholds.test] excessive_fields_max = 0` and role-assigned fixture asserting no `excessive_fields` violation — covers "Test role disables excessive-field check" end-to-end. GREEN: fix role merge or wiring if needed. Verify: `cargo test cli_test`.
- [ ] 7.3 Verify SARIF output for the 7.1 fixture still validates against SARIF 2.1.0 and carries `rule_id` `excessive_fields`/`primitive_field` with the fixture line region — covers "Finding is consistent across output formats" (SARIF leg). Verify: existing SARIF schema test extended, `cargo test`.

## 8. Final integration

- [ ] 8.1 Run `cargo test && cargo fmt --check && cargo clippy -- -D warnings`; fix any fallout. Verify: all three pass.
- [ ] 8.2 Run `ah check` for spec-test correspondence and dogfood `pretender check . --staged` before committing. Verify: both clean.
- [ ] 8.3 Update `CHANGELOG.md` under Unreleased with the new checks, config keys, and defaults. Verify: entry present.
- [ ] 8.4 Update `docs/src/configuration.md`: add `excessive_fields_max`, `primitive_field_check`, and `domain_patterns` rows to the `[thresholds.app]` table that already lists `bool_cluster_max` / `primitive_param_check`, and list the new rule ids wherever rules are enumerated. Note in the docs that 10 is a provisional default pending calibration (pretender-uwz), that `primitive_field` is opt-in/experimental, and that generated/FFI-heavy paths should exclude or disable the check via role globs. Verify: documented defaults match `config.rs` defaults.
- [ ] 8.5 Refresh the dogfood baseline: run `pretender check .` (full worktree), `--adopt` new `excessive_fields` findings into `pretender-baseline.json`, record the new-known count in the PR description. Verify: `pretender check . --staged` gate green afterward.

## Workflow follow-up

- Open PR from a feature branch (no direct pushes to main) after review requirements are satisfied.
- Archive the change via openspec archive once implementation and review are complete; verify archived specs.
