# Project Context

## Purpose

**Pretender** is a language-agnostic code quality CLI that catches code that *pretends* to be fine — tests with no assertions, functions with high ABC scores, structural duplication that line-diff misses. It runs as a fast pre-commit hook (<2s), generates its own CI integration, and emits SARIF for IDE and GitHub Code Scanning integration.

Binary: `pretender` · Config: `pretender.toml` · License: Apache 2.0

## Tech Stack

- **Rust** — single-binary distribution, tree-sitter Rust bindings, <10ms startup
- **tree-sitter** — CST parsing for 100+ grammars; `.scm` query files per language
- **tree-sitter-loader** — loads tree-sitter grammars and query files used by data-only language plugins
- **clap** — CLI argument parsing
- **serde + toml** — config parsing
- **serde_sarif** — SARIF 2.1.0 emission
- **git2** — staged-file detection, diff-base resolution
- **rayon** — parallel file processing
- **indicatif** — progress bars
- **miette** — pretty diagnostic errors
- **thiserror** — typed library errors (used with `anyhow` for CLI propagation)

## Project Conventions

### Code Style

- Rust 2021 edition
- `rustfmt` for formatting (default settings)
- `clippy` for linting (deny warnings in CI)
- Error types: `thiserror` for library errors, `anyhow` for CLI error propagation
- No `unwrap()` in library code; use `?` or explicit handling
- Prefer `&str` / `Cow<str>` over `String` at boundaries

### Architecture Patterns

The tool has exactly two layers that matter:

1. **Universal code model** — every metric operates on this (`CodeUnit`, `Block`, `Branch`, `Node`)
2. **Per-language adapter** — tree-sitter `.scm` query files that populate the model

Metrics are **pure functions** over the universal model:
```rust
fn cyclomatic(u: &CodeUnit) -> u32 { 1 + count_branches(&u.body) }
```

Plugin system is **file-system based** — language plugins are data-only `.scm` files + `plugin.toml` manifests. No compilation or native code execution is required for new languages.

Three operating modes: `guidance` (report only) → `tiered` (green/yellow/red) → `gate` (hard fail).

Code **roles** are first-class: `app`, `library`, `test`, `script`, `generated`, `vendor`. Test code has *stricter* cyclomatic/nesting/cognitive limits and *looser* length/duplication limits. Detected via explicit pragma > path globs > heuristics (pragma wins when present).

### Testing Strategy

- Unit tests for each metric function (pure functions are trivially testable)
- Integration tests: feed real source files through the full pipeline, assert on metric values
- Snapshot tests (insta) for CLI output format regression
- No mocking of file I/O — use temp dirs with real files
- No network calls in tests — grammar downloads must use a pre-seeded local cache dir in test fixtures
- `cargo test` must pass before commit; `cargo clippy -- -D warnings` in CI

### Git Workflow

- Single `main` branch
- Conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`, `chore:`
- No force-push to main
- Tag releases as `v0.x.y`; use `cargo dist` for binary distribution

## Domain Context

### Universal Code Model

Every language maps its AST to these types:
- `Module` — file-level container (path, language, line counts, units, imports)
- `CodeUnit` — function/method/lambda/constructor (name, kind, span, parameters, body)
- `Block` — nested scope (span, nesting depth, child nodes)
- `Node` — Statement | Branch | NestedBlock | Call
- `Branch` — control flow point with `BranchKind` and nesting depth (for cognitive complexity)

### Key Metrics

| Metric | Formula | Threshold |
|--------|---------|-----------|
| Cyclomatic | 1 + branch count | ≤10 app, ≤3 test |
| Cognitive | Σ(branch_weight × (1 + nesting)) | ≤15 app, ≤5 test |
| ABC | √(A² + B² + C²) with smell weights | per-file sorted |
| Function lines | span.lines() | ≤40 app, ≤80 test |
| Nesting max | max block depth | ≤3 app, ≤2 test |
| Params | parameters.len() | ≤4 app, ≤2 test |
| Min assertions | count assertion captures | ≥1 test only |
| Duplication | normalised AST subtree hash collision rate | ≤5% app, ≤30% test |

### CLI Commands

```
pretender init                    # interactive config wizard
pretender check [paths]           # fast pass/fail (non-zero exit in gate mode)
pretender complexity [paths]      # ABC + smell weights, sorted worst-first
pretender duplication [paths]     # structural clone detection
pretender mutation [paths]        # mutation testing wrapper
pretender report                  # TUI/HTML report from last run
pretender hooks install           # writes .git/hooks/pre-commit
pretender ci generate <provider>  # github | gitlab | circle | generic
pretender plugins list|add|remove
pretender explain <metric>        # metric description + threshold citation
```

### Output Formats

`human` (default terminal) · `json` (machine pipelines) · `sarif` (GitHub Code Scanning, IDE squiggles) · `junit` (CI reporters) · `markdown` (PR comments, step summaries)

## Important Constraints

- Hook must complete in **<2s** on a normal commit — diff-only mode is critical
- Tree-sitter is the **core** parser (not LSP). LSP is optional, V2-only, for coupling/dead-code
- Language plugins ship as data-only `.scm` query files plus `plugin.toml` manifests — no native dynamic loading or recompilation
- Top 10 languages compiled into the binary: Python, JavaScript, TypeScript, Rust, Go, Java, Ruby, C, C++, C# — others downloaded on demand with checksum pinning
- SARIF output must be valid SARIF 2.1.0 (OASIS standard) for GitHub Code Scanning compat
- Single-binary distribution via `cargo dist`
- Duplication detection: within-file in V0/V1, cross-file behind a flag in V1

## External Dependencies

- **tree-sitter grammars** — fetched at build time or on demand; pinned by rev
- **Stryker / PIT / mutmut / cargo-mutants** — wrapped by `pretender mutation`, not reimplemented
- **eslint / ruff / clippy / staticcheck** — wrapped as metric plugins via `plugin.toml` command spec
- **cargo dist** — binary release distribution (GitHub Releases + Homebrew tap)
- **GitHub Code Scanning / SARIF** — OASIS SARIF 2.1.0 for PR annotation integration
