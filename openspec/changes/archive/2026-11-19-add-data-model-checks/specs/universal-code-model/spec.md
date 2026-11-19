# universal-code-model spec delta: type definitions enter the model

## ADDED Requirements

### Requirement: Type Definition Model

The system SHALL represent each struct, class, or record definition captured by a language adapter as a `TypeDefinition` on the containing `Module`'s `type_definitions` list, with name, kind (struct, class, or record), span, ordered fields, and language. Each captured field SHALL carry a name, declared type when present in the source, and span. `TypeDefinition` SHALL be distinct from `CodeUnit`; function-like members of a type remain `CodeUnit` entries and SHALL NOT be duplicated as fields.

#### Scenario: Struct maps to type definition

- **WHEN** a language adapter captures a struct definition with two fields
- **THEN** the universal model contains one `TypeDefinition` on that file's `Module` with kind struct and two ordered fields

#### Scenario: Members are not duplicated

- **WHEN** a class definition contains both fields and methods
- **THEN** the model contains one `TypeDefinition` (fields only) plus separate `CodeUnit` entries for each method

### Requirement: Type Captures Are Optional

Language adapters SHALL NOT be required to provide type captures. A query file without type captures SHALL yield an empty `type_definitions` list for that language's modules, and the model SHALL treat empty as valid rather than as a parse failure.

#### Scenario: Adapter without type captures stays valid

- **WHEN** a language's query file provides function captures but no type captures
- **THEN** its modules parse successfully with an empty `type_definitions` list

## MODIFIED Requirements

### Requirement: Tree-Sitter Query Contract

Each implemented language adapter SHALL provide a `.scm` query file whose captures populate the universal model. The current MVP query engine requires captures for `@function.definition`, `@function.name`, `@function.parameters`, and `@function.body`; it also consumes optional branch, `@call`, `@call.callee`, `@assign`, and `@assert.*` captures.

The query engine SHALL additionally consume optional type-definition captures: `@type.definition`, `@type.name`, `@type.field`, `@type.field.name`, and `@type.field.type`. Type captures SHALL populate the `type_definitions` list of the containing `Module`; when absent, the list SHALL be empty and parsing SHALL still succeed.

Assertion captures SHALL contribute to the per-code-unit assertion count used by role-specific threshold checks.

#### Scenario: Function captures populate code unit
- **WHEN** a query captures `@function.definition`, `@function.name`, `@function.parameters`, and `@function.body`
- **THEN** the adapter creates the corresponding `CodeUnit`

#### Scenario: Type captures populate type definition
- **WHEN** a query captures `@type.definition`, `@type.name`, and `@type.field` with `@type.field.name` captures
- **THEN** the adapter creates the corresponding `TypeDefinition` with its captured fields
