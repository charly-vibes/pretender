# Design: add-data-model-checks

## Context

The universal model (`pretender/src/model.rs`) captures callables as `CodeUnit`; type definitions are invisible. `inheritance_depth.rs` extracts class declarations with ad-hoc text regexes per language — outside the model, single-file only, and unusable by other checks. The tree-sitter query engine consumes one `metrics.scm` per language with `@function.*` captures; type nodes are currently discarded. App-level checks (`bool_cluster`, `primitive_param`, `inheritance_depth`) run in `main.rs` per file and push `ViolationReport { metric, actual, limit }` onto `FileReport.violations`; SARIF emission maps these to `rule_id` + region with a caller-supplied `start_line`. `ViolationReport` carries no span today.

## Goals / Non-Goals

**Goals**
- Type definitions become first-class, query-populated entities on the universal model.
- Two new app-threshold checks (`excessive_fields`, `primitive_field`) flow through the existing violations pipeline and output formats unchanged.
- Language adapters can adopt type captures incrementally; absence is valid.

**Non-Goals**
- Refactoring `inheritance_depth.rs` onto the new entity (follow-up).
- Anemic-model, value-object, and interface/enum checks.
- Cross-file type resolution (this change is per-file, matching existing checks).
- New commands or output-format schemas.

## Decisions

**D1: New `TypeDefinition` entity, not a new `UnitKind`.**
Alternative rejected: adding `Struct`/`Class` to `UnitKind` would force every existing consumer (metrics, cluster/mock detection, role checks) to filter out non-callable units. A separate `type_definitions: Vec<TypeDefinition>` on `Module` leaves `CodeUnit` semantics untouched — zero existing check changes. Field shape: `{ name, type_name: Option<String>, span }`; kind enum `{ Struct, Class, Record }`.

**D2: Optional `@type.*` captures in the query contract.**
Mirrors how imports are handled today (empty list is valid). The engine gains a capture family (`@type.definition`, `@type.name`, `@type.field`, `@type.field.name`, `@type.field.type`) and populates `Module.type_definitions`. Alternative rejected: mandating captures for all languages — that would force awkward mappings (e.g., Ruby `attr_accessor` is a method call, not a field node) and couple this change to grammar quirks of all 13 languages at once.

**D3: Kind mapping is per-language and best-effort.**
`Record` is reserved for languages where the record declaration is its own node: Java/C# `record` and Clojure `defrecord`. Decorator-based refinement is not expressible in the defined capture family, so Python `@dataclass` ships as `Class` — descoped; revisiting with a `@type.kind` capture is a follow-up candidate. Structs → `Struct` (Rust, Go, C, C++, Julia `struct`/`mutable struct`); classes → `Class` (C++/Java/C#/TS/JS/Python/Ruby classes, Clojure `deftype`). Interfaces, traits, enums, and R's function-based class systems are not captured. Any language that ends up without captures ships with empty `type_definitions` (spec-compliant); the filled matrix lands in the appendix below via tasks 6.7.

**D4: Checks live in a new pure module (`data_model.rs`), not inside `primitive_obsession.rs`.**
`analyze(type_defs, excessive_fields_max, primitive_field_check, domain_patterns) -> Vec<...>` — pure function over the model, same pattern as existing analyses, independently unit-testable. Wiring in `main.rs` sits beside the other app-threshold checks and pushes `ViolationReport`s.

**D5: `ViolationReport` gains an optional `line: Option<u32>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`**
The project rule requires findings to keep rule ID, file, and line range across formats, but today's struct cannot express a line — SARIF regions get a caller-supplied file-level start line. Data-model findings need the type-definition (excessive_fields) or field (primitive_field) line. An additive `Option` field is backward-compatible for JSON consumers — `skip_serializing_if` keeps existing output byte-identical when `None` (plain `serde(default)` alone would emit `"line": null`); the SARIF closure uses it when present, else the existing default. Alternative rejected: file-level-only findings (violates the finding-consistency rule).

**D6: New `domain_patterns: Vec<String>` key in `[thresholds.app]`.**
There is no existing configured domain-pattern list to reuse — the pattern argument for `primitive_param_check` is fed an empty literal in `main.rs` (`extra_domain_patterns` exists only as an analysis-parameter name). `primitive_field_check` therefore gets its own `domain_patterns` key (default empty) rather than pretending to reuse config that does not exist. Pointing `primitive_param_check` at the same key is out of scope here.

**D7: Thresholds — `excessive_fields_max: u32` default 10, `primitive_field_check: bool` default false**, in `AppThresholds` with serde defaults, role-overridable via existing table merge. `0` disables `excessive_fields` (matches `bool_cluster_max`/`inheritance_depth_max` conventions). The default 10 is a **provisional policy choice, not an ecosystem precedent**: documented field-count defaults are 7 (pylint too-many-instance-attributes), 15 (PMD TooManyFields, non-static non-final), 20 (SonarQube S1820) — no tool ships 10. Calibrate against a repository/corpus distribution before treating the value as a quality boundary (follow-up pretender-uwz); the spec pins behavior, not provenance.

## Risks / Trade-offs

- [Query engine complexity: nested field captures inside type bodies] → Scoping rule: fields bind to the innermost enclosing `@type.definition`; the engine test suite covers nesting (anonymous inner types, methods inside classes) before any adapter work.
- [`excessive_fields` false positives on legitimate data holders (DTOs, config structs, generated code)] → Field count is a size signal, not a responsibility verdict (survey: keep field counting separate from responsibility detection). Default 10 is provisional (tighter than PMD's 15 / SonarQube's 20, looser than pylint's 7) and the check is threshold-driven; users disable per role via `[thresholds.<role>]` or set `0`. Documented in the capability spec, not hardcoded exemptions; corpus calibration tracked as pretender-uwz.
- [`primitive_field` noise from FFI/layout-sensitive code (C, Rust `repr(C)`)] → Check is off by default; per-role overrides cover generated/FFI directories via role globs.
- [Tree-sitter grammar drift across 9 languages] → Each adapter change lands with a per-language round-trip test against a fixture; a language failing its fixture blocks only its own captures, not others'.
- [`ViolationReport.line` ripples through report/markdown/html emitters] → Emitters that ignore the field keep working (serde default); only JSON/SARIF gain the value in this change.

## Migration Plan

Additive change; no config migration (unknown-key tolerance already covers older `pretender.toml` files, and new keys have defaults). Rollback = revert; consumers of JSON/SARIF see at most a new optional `line` field and new rule ids, both ignorable. Rollout is baseline-gated: new `excessive_fields` findings on existing code land in the dogfood baseline (survey §4.1 endorses baseline-of-stable-ids for pretender); tasks §8 refresh the baseline and record the new-known count. Rule ids were fixed at proposal time (`excessive_fields`, not `god_object`) so the field-count claim never masquerades as god-class detection — renames after merge would break SARIF rule-id compatibility.

## Evidence Base

Distilled from the 13-provider survey in `~/cotorreo/raw/data-model-checks-survey/` (2026-11-18): adopt/adapt/avoid/defer recommendations, documented threshold precedents (pylint 7, PMD 15, SonarQube 20, NDepend 15), rule-maturity conventions (default / opt-in / experimental), and the survey's core cautions (field count ≠ responsibility; `String email` is not automatically primitive obsession; stable finding ids for baseline adoption).

## Open Questions

None blocking. Ruby `attr_accessor` field mapping and Python `@dataclass` kind refinement may land later without spec impact (zero captures / `Class` are both valid today).

## Appendix: Language Coverage Matrix

Filled in by task 6.7; one row per built-in language adapter (13):

| Language | Type nodes | Kinds captured | Fixture |
|---|---|---|---|
| C | _TBD (task 6.5)_ | _TBD_ | _TBD_ |
| C++ | _TBD (task 6.5)_ | _TBD_ | _TBD_ |
| C# | _TBD (task 6.3)_ | _TBD_ | _TBD_ |
| Clojure | _TBD (task 6.6)_ | _TBD_ | _TBD_ |
| Go | _TBD (task 6.1)_ | _TBD_ | _TBD_ |
| Java | _TBD (task 6.3)_ | _TBD_ | _TBD_ |
| JavaScript | _TBD (task 6.4)_ | _TBD_ | _TBD_ |
| Julia | _TBD (task 6.6)_ | _TBD_ | _TBD_ |
| Python | _TBD (task 6.2)_ | _TBD_ | _TBD_ |
| R | _TBD (task 6.6)_ | _TBD_ | _TBD_ |
| Ruby | _TBD (task 6.2)_ | _TBD_ | _TBD_ |
| Rust | _TBD (task 6.1)_ | _TBD_ | _TBD_ |
| TypeScript | _TBD (task 6.4)_ | _TBD_ | _TBD_ |
