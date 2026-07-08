# C# Language Plugin

Plugin for parsing C# source files using `tree-sitter-c-sharp`.

## Queries

- `metrics.scm` — captures function definitions (`method_declaration`,
  `constructor_declaration`, `destructor_declaration`,
  `local_function_statement`), branches, calls, and assignments
- `plugin.toml` — configuration with extension `.cs`, branch weights,
  assertion patterns, and smell weights