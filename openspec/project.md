# Project Context

## Purpose

**Pretender** is a language-aware code quality CLI that flags structural issues such as high complexity, missing assertions in tests, and risky call patterns.

Binary: `pretender` · Config: `pretender.toml` · License: Apache 2.0

## Tech Stack

- **Rust**
- **tree-sitter** grammars compiled into the binary for current supported languages
- **clap** for CLI parsing
- **serde + toml** for config parsing
- **serde_sarif** for SARIF 2.1.0 emission
- **rayon** for parallel file processing
- **miette + thiserror + anyhow** for diagnostics and error propagation

## Current MVP Surface

Implemented commands:
- `pretender init`
- `pretender check [paths...]`
- `pretender complexity <path>`
- `pretender report`
- `pretender hooks install|uninstall`
- `pretender ci generate github`

Reserved commands:
- `pretender duplication`
- `pretender mutation`
- `pretender plugins list|add|remove`
- `pretender explain <metric>`

Current `check` formats:
- `human`
- `json`
- `sarif`

Current `report` formats:
- `human`
- `markdown`
- `html`

Not yet implemented in `check`:
- `--staged`
- `--diff-only`
- `--diff-base <ref>`
- `--format junit|markdown`

## Project Conventions

### Code Style

- Rust 2021 edition
- `rustfmt` for formatting
- `clippy` with warnings denied in CI
- No `unwrap()` in library-style code paths unless tests/fixtures justify it

### Architecture Patterns

The tool has two core layers:
1. **Universal code model** consumed by metrics
2. **Per-language adapters** that populate the model from tree-sitter queries

Metrics are pure functions over the universal model.

### Testing Strategy

- Unit tests for metric and config logic
- Integration tests that execute the CLI against fixture files
- No network calls in tests
- `cargo test`, `cargo fmt --check`, and `cargo clippy -- -D warnings` should pass before commit

## Current Language Support

Built-in parsers currently handle:
- C
- C++
- Go
- Java
- JavaScript
- Python
- Ruby
- Rust
- TypeScript

## Important Constraints

- Hook generation currently installs a repo-wide `pretender check .` shim until diff filtering lands
- SARIF output must remain compatible with SARIF 2.1.0
- Plugin manifests are parsed, but plugin runtime loading is not yet implemented
