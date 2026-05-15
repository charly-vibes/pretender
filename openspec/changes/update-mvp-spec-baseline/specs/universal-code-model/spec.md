## MODIFIED Requirements

### Requirement: Module Model

The system SHALL represent each successfully parsed source file as one `Module` containing path, language, whole-file span, total/code/comment line counts, code units, and imports. In the current MVP, imports are emitted as an empty list and only the Python adapter is registered by the parser registry.

#### Scenario: One Python file maps to one module
- **WHEN** a supported Python source file is parsed successfully
- **THEN** the engine emits one `Module` for that file

### Requirement: Tree-Sitter Query Contract

Each implemented language adapter SHALL provide a `.scm` query file whose captures populate the universal model. In the current MVP, the query engine requires captures for `@function.definition`, `@function.name`, `@function.parameters`, and `@function.body`; it also consumes optional branch, `@call`, `@call.callee`, and `@assign` captures.

Assertion-related captures MAY exist in query files, but the current MVP query engine does not use them to populate the universal model or enforce assertion-count checks.

#### Scenario: Function captures populate code unit
- **WHEN** a query captures `@function.definition`, `@function.name`, `@function.parameters`, and `@function.body`
- **THEN** the adapter creates the corresponding `CodeUnit`

### Requirement: Model Invariants

The system SHALL preserve these invariants: code-unit body nesting starts at 0; nested blocks increment nesting by 1; branches carry nesting depth at capture time; languages without explicit visibility default `is_exported` to false except for the current MVP Python behavior, where names beginning with `_` are not exported and other names are exported.

For the current MVP parse-error path, files with tree-sitter parse errors SHALL emit a warning diagnostic and an otherwise-empty `Module` with no code units.

#### Scenario: Parse error returns empty module with warning
- **WHEN** tree-sitter reports a parse error for a Python file
- **THEN** the system emits a warning diagnostic and returns a `Module` with no `CodeUnit` values

### Requirement: Metric Functions

The system SHALL compute metrics as pure functions over the universal model. Cyclomatic complexity SHALL be `1 + branch count`. Cognitive complexity SHALL sum branch cognitive weight multiplied by `1 + nesting_at`. Function lines SHALL use inclusive span lines. Parameter count SHALL be parameter length. Maximum nesting SHALL be the maximum block nesting value. ABC SHALL compute `sqrt(A^2 + B^2 + C^2)` using counted assignments, weighted branches, and weighted calls.

In the current MVP, parsed `CallSite` values are assigned a default `smell_weight` of `1.0` by the parser.

#### Scenario: Cyclomatic adds one to branches
- **WHEN** a code unit has 4 branch nodes
- **THEN** cyclomatic complexity is 5

## ADDED Requirements

### Requirement: Current Adapter Surface

The current MVP SHALL register a Python adapter that maps `.py` files into the universal model using the built-in Python query file.

#### Scenario: Python adapter is available
- **WHEN** a `.py` file is analysed
- **THEN** the parser registry resolves the Python adapter for that file extension

## REMOVED Requirements

### Requirement: Model Versioning and Compatibility
**Reason**: Runtime enforcement of model-version compatibility and plugin minimum-version negotiation is not implemented in the current MVP.
**Migration**: Restore when plugin loading and compatibility checks exist.
