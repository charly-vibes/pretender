# Universal Code Model

## Purpose

Defines the language-neutral model that all Pretender metrics operate on, plus the tree-sitter query contract, model invariants, metric formulas, and compatibility rules.

## Requirements

### Requirement: Supporting Types

The system SHALL represent source spans, parameters, call sites, and languages in a language-neutral form. `Span.lines()` SHALL be inclusive and SHALL require `end_line >= start_line`. `CallSite` SHALL include a callee name, span, and default ABC smell weight of `1.0`.

#### Scenario: Span line count is inclusive
- **WHEN** a span has `start_line = 3` and `end_line = 5`
- **THEN** `Span.lines()` returns 3

### Requirement: Module Model

The system SHALL represent each successfully parsed source file as one `Module` containing path, language, whole-file span, total/code/comment line counts, code units, and imports. In the current MVP, imports are emitted as an empty list by the built-in query engine adapters.

#### Scenario: One file maps to one module
- **WHEN** a supported source file is parsed successfully
- **THEN** the engine emits one `Module` for that file

### Requirement: Code Unit Model

The system SHALL represent each function, method, lambda with block body, constructor, or initializer as a `CodeUnit` with name, kind, span, parameters, body block, and exported flag.

#### Scenario: Function maps to code unit
- **WHEN** a language adapter captures a function definition
- **THEN** the universal model contains a `CodeUnit` for that function

### Requirement: Block and Node Model

The system SHALL represent nested scopes as `Block` values with zero-based nesting depth from the enclosing `CodeUnit` body root. `Node` SHALL represent statements, branches, nested blocks, and calls.

#### Scenario: Root body nesting is zero
- **WHEN** a `CodeUnit` body is created
- **THEN** its root block has `nesting = 0`

### Requirement: Branch Model

The system SHALL represent control-flow points as `Branch` values with kind, span, nesting depth at capture time, and optional logical sequence ID. Branch kinds SHALL include if, else-if, switch case, loop, catch, ternary, logical-and, and logical-or.

#### Scenario: Logical sequence IDs are scoped to code unit
- **WHEN** logical operators are captured in two different code units
- **THEN** their sequence IDs are unique only within each enclosing code unit

### Requirement: Tree-Sitter Query Contract

Each implemented language adapter SHALL provide a `.scm` query file whose captures populate the universal model. The current MVP query engine requires captures for `@function.definition`, `@function.name`, `@function.parameters`, and `@function.body`; it also consumes optional branch, `@call`, `@call.callee`, `@assign`, and `@assert.*` captures. Assertion captures SHALL contribute to the per-code-unit assertion count used by role-specific threshold checks.

#### Scenario: Function captures populate code unit
- **WHEN** a query captures `@function.definition`, `@function.name`, `@function.parameters`, and `@function.body`
- **THEN** the adapter creates the corresponding `CodeUnit`

### Requirement: Type-Definition Captures

The query engine SHALL additionally consume optional type-definition captures: `@type.definition`, `@type.name`, `@type.field`, `@type.field.name`, and `@type.field.type`. Type captures SHALL populate the `type_definitions` list of the containing `Module`; when absent, the list SHALL be empty and parsing SHALL still succeed.

#### Scenario: Type captures populate type definition
- **WHEN** a query captures `@type.definition`, `@type.name`, and `@type.field` with `@type.field.name` captures
- **THEN** the adapter creates the corresponding `TypeDefinition` with its captured fields

### Requirement: Model Invariants

The system SHALL preserve these invariants: code-unit body nesting starts at 0; nested blocks increment nesting by 1; branches carry nesting depth at capture time; files with tree-sitter parse errors emit a warning diagnostic and return an otherwise-empty `Module`; identical logical operator sequences at the same nesting level count once for cognitive complexity.

#### Scenario: Parse error returns empty module with warning
- **WHEN** tree-sitter reports a parse error for a source file
- **THEN** the system emits a warning diagnostic and returns a `Module` with no `CodeUnit` values

### Requirement: Export Heuristic

In the current MVP query-engine-backed adapters, the system SHALL treat code units whose names begin with `_` as not exported and all other named units as exported.

#### Scenario: Underscore-prefixed unit is not exported
- **WHEN** a query-engine-backed adapter parses a code unit named `_helper`
- **THEN** the unit is treated as not exported, while a unit named `helper` is treated as exported

### Requirement: Metric Functions

The system SHALL compute metrics as pure functions over the universal model. Cyclomatic complexity SHALL be `1 + branch count`. Cognitive complexity SHALL sum branch cognitive weight multiplied by `1 + nesting_at`. Function lines SHALL use inclusive span lines. Parameter count SHALL be parameter length. Maximum nesting SHALL be the maximum block nesting value. ABC SHALL compute `sqrt(A^2 + B^2 + C^2)` using counted assignments, weighted branches, and weighted calls.

#### Scenario: Cyclomatic adds one to branches
- **WHEN** a code unit has 4 branch nodes
- **THEN** cyclomatic complexity is 5

### Requirement: Call Site Weighting

In the current MVP, the parser SHALL assign every parsed `CallSite` a default `smell_weight` of `1.0`, with smell-specific weighting applied later during metric evaluation.

#### Scenario: Parsed call site gets default weight
- **WHEN** the parser extracts a `CallSite` from source
- **THEN** it carries the default `smell_weight` of `1.0` until smell-specific weighting applies during metric evaluation

### Requirement: Current Adapter Surface

The current MVP SHALL register built-in adapters for C, C++, Go, Java, JavaScript, Python, Ruby, Rust, and TypeScript-family source files.

#### Scenario: TypeScript adapter is available
- **WHEN** a `.ts` file is analysed
- **THEN** the parser registry resolves the TypeScript adapter for that file extension

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
