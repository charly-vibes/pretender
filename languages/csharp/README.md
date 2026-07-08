# C# Language Plugin

Plugin for parsing C# source files using `tree-sitter-c-sharp`.

## Version Pin

`tree-sitter-c-sharp` is pinned to `=0.23.0` in `pretender/Cargo.toml` because
v0.23.5+ uses language ABI version 15, which is incompatible with the project's
current `tree-sitter` runtime (v0.23.2, supports up to ABI 14).

When the main `tree-sitter` crate is upgraded to 0.24+, this pin can be relaxed
to `"0.23"` or later.

## Queries

- `metrics.scm` — captures function definitions (`method_declaration`,
  `constructor_declaration`, `destructor_declaration`,
  `local_function_statement`), branches, calls, and assignments
- `plugin.toml` — configuration with extension `.cs`, branch weights,
  assertion patterns, and smell weights