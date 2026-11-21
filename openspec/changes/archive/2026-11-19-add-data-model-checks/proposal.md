# Proposal: add-data-model-checks

## Why

Pretender's universal code model only captures functions, methods, lambdas, constructors, and initializers (`CodeUnit`). Type definitions — structs, classes, records — are invisible to the model, so data-model-shaped complexity drift (god objects accumulating fields, domain values stored as bare primitives) is undetected before merge. Primitive obsession is already flagged at the *parameter* level only (`bool_cluster`, `primitive_param`); the definitions those parameters feed into are never evaluated. Closing this gap directly serves the falsifiable value proposition: reducing undetected structural quality regressions flagged by CI.

## What Changes

- Extend the universal model with a `TypeDefinition` entity (name, kind: struct/class/record, span, fields with names and types, language) stored on `Module` as a new `type_definitions` list. `CodeUnit` and `UnitKind` are unchanged.
- Extend the per-language `metrics.scm` query contract with optional captures (`@type.definition`, `@type.name`, `@type.field`, `@type.field.name`, `@type.field.type`); adapters that provide no type captures simply produce zero type definitions — this is spec-compliant, not an error.
- Add a new pure analysis module over type definitions with two checks wired into `pretender check` (surface: `check` pipeline; declaration kind: structural/complexity findings, same `Diagnostic` path as existing checks):
  - `excessive_fields` — type definition whose field count meets or exceeds a configured max (metric name: `excessive_fields`, threshold: `excessive_fields_max`, default 10 — a provisional policy default pending corpus calibration, not an ecosystem precedent, `0` disables)
  - `primitive_field` — field whose declared type is a bare primitive despite a domain-like name (metric name: `primitive_field`, toggle: `primitive_field_check`, default off, mirroring `primitive_param_check`)
- New config keys under the existing `[thresholds.app]` table (`excessive_fields_max`, `primitive_field_check`, `domain_patterns`), role-overridable per the existing Configuration Schema requirement — additive only, unknown-key forward compatibility preserved. `domain_patterns` is a new list (default empty); no configured domain-pattern list exists today.
- Findings keep rule ID, file, line range, and severity, so human, JSON, and SARIF 2.1.0 output stay consistent. No breaking changes; no new commands.

## Capabilities

### New Capabilities
- `data-model-checks`: threshold-based structural checks evaluated over type definitions captured in the universal model (type field count, field-level primitive-obsession candidates), including finding shape, defaults, disable semantics, and role threshold application.

### Modified Capabilities
- `universal-code-model`: adds a Type Definition Model requirement (entity shape, placement on `Module`, invariant that type captures are optional per language) and extends the Tree-Sitter Query Contract with the optional `@type.*` captures.

## Impact

- **Universal model**: `pretender/src/model.rs` — new `TypeDefinition`/`Field` types; `Module` gains `type_definitions`.
- **Adapters**: `pretender/languages/*/metrics.scm` (all 13 built-in language adapters — C, C++, C#, Clojure, Go, Java, JavaScript, Julia, Python, R, Ruby, Rust, TypeScript) and `pretender/src/engine.rs` capture handling. Natural node coverage: Rust `struct_item`, Python `class_definition` (dataclasses ship as Class — decorator-based kind refinement is out of scope), Go `type_declaration` struct specs, Java/C# class and record declarations, TS/JS class declarations, C/C++ struct/class, Julia `struct`/`mutable struct`, Clojure `defrecord`/`deftype`, Ruby classes (attr_* are method calls in the grammar — expect zero field captures per design D3), R (likely zero captures — class systems are function-based). Zero type definitions for any language is acceptable and spec-compliant.
- **Checks pipeline**: new analysis module (pure function over `&[TypeDefinition]`, mirroring `primitive_obsession.rs`); wiring in `pretender/src/main.rs` alongside existing app-threshold checks; findings flow through existing gate/output machinery.
- **Config**: `pretender/src/config.rs` `AppThresholds` gains two additive keys with defaults.
- **Tests**: unit tests for the new analysis (TDD), adapter/engine round-trip tests, CLI integration test with a fixture containing an excessive-field struct and primitive fields.
- **Out of scope** (recorded assumptions): value-object opportunity detection, anemic-model detection, refactoring `inheritance_depth.rs` onto the new entity, and interface/enum extraction.
