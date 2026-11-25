# C# Language Plugin

Plugin for parsing C# source files using `tree-sitter-c-sharp`.

## Queries

- `metrics.scm` — captures function definitions (`method_declaration`,
  `constructor_declaration`, `destructor_declaration`,
  `local_function_statement`), branches, calls, and assignments
- `plugin.toml` — configuration with extension `.cs`, branch weights,
  assertion patterns, and smell weights

## Known unsupported metrics

- **`min_assertions`** — *Not implemented yet*: `metrics.scm` has no
  `@assert.*` capture, so unit assertion totals are always 0 (the
  `[assertions]` patterns serve rule scopes only). No ticket filed yet.
- **Unwrap density** — *Not implemented yet*: the engine's risky
  error-handling detector covers Rust, Python, JavaScript/TypeScript, Java,
  and Go only; C# has no matcher. No ticket filed yet.