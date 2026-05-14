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

The system SHALL represent each source file as one `Module` containing path, language, whole-file span, total/code/comment line counts, code units, and imports. Imports SHALL be populated only when LSP support is enabled.

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

Each supported language SHALL provide a `.scm` query file whose captures populate the universal model. Required capture conventions SHALL include function definition/name/parameters/body, branch captures, `@call`, `@assign`, and `@assert.*` assertion captures.

#### Scenario: Function captures populate code unit
- **WHEN** a query captures `@function.definition`, `@function.name`, `@function.parameters`, and `@function.body`
- **THEN** the adapter creates the corresponding `CodeUnit`

### Requirement: Model Invariants

The system SHALL preserve these invariants: code-unit body nesting starts at 0; nested blocks increment nesting by 1; branches carry nesting depth at capture time; languages without explicit visibility default `is_exported` to false; Python and Ruby units whose names start with `_` are not exported; expression lambdas are statements rather than call sites; files with tree-sitter parse errors are skipped entirely and emit diagnostics; identical logical operator sequences at the same nesting level count once for cognitive complexity.

#### Scenario: Parse error skips file
- **WHEN** tree-sitter reports a parse error for a file
- **THEN** the system emits a diagnostic and does not emit a partial `Module`

### Requirement: Metric Functions

The system SHALL compute metrics as pure functions over the universal model. Cyclomatic complexity SHALL be `1 + branch count`. Cognitive complexity SHALL sum branch cognitive weight multiplied by `1 + nesting_at`. Function lines SHALL use inclusive span lines. Parameter count SHALL be parameter length. Maximum nesting SHALL be the maximum block nesting value. ABC SHALL compute `sqrt(A^2 + B^2 + C^2)` using counted assignments, weighted branches, and weighted calls.

#### Scenario: Cyclomatic adds one to branches
- **WHEN** a code unit has 4 branch nodes
- **THEN** cyclomatic complexity is 5

### Requirement: Model Versioning and Compatibility

The universal model SHALL use semantic versioning. Removing fields or changing capture semantics SHALL require a major version bump. Adding branch kinds or optional fields SHALL be a minor version bump. Language adapters SHALL declare their minimum supported model version in `plugin.toml`.

#### Scenario: Plugin declares model version
- **WHEN** a language plugin is loaded
- **THEN** the engine verifies that its declared minimum model version is compatible
