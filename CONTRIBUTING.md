# Contributing

## Prerequisites

- **Rust** stable toolchain — install via [rustup.rs](https://rustup.rs/)
- **just** — install via [just.systems](https://just.systems/) or `cargo install just`
- **lefthook** — install via [lefthook.dev](https://lefthook.dev/) or `brew install lefthook`
- **Git** ≥ 2.x

## Setup

```sh
git clone https://github.com/charly-vibes/pretender
cd pretender
just build
lefthook install   # activates pre-commit and pre-push hooks
```

## Development workflow

```sh
just test          # run all tests
just type-check    # cargo check (fast)
just lint          # cargo clippy
just fmt           # auto-format
just ci            # full gate: fmt-check + type-check + lint + test
```

## Quality gates

Hooks enforce these gates automatically:

| Hook | Checks |
|------|--------|
| pre-commit | `cargo check`, `cargo clippy` |
| pre-push | `cargo fmt --check`, `cargo test` |

CI runs the same gate on every push and pull request.

## Structural standard (self-hosted dogfooding)

Pretender checks its own source with itself. Thresholds live in `pretender.toml` (see defaults in `pretender/src/config.rs`); roles apply by path:

| Role | Key limits (defaults) |
|------|----------------------|
| app (`src/**`) | `cyclomatic_max: 10`, `cognitive_max: 15`, `function_lines_max: 30`, `file_lines_max: 400`, `nesting_max: 3`, `params_max: 4`, `abc_max: 30`, `duplication_pct_max: 5`, `unwrap_max: 5` |
| test (`tests/**`) | `cyclomatic_max: 3`, `function_lines_max: 80`, `nesting_max: 2`, `params_max: 2`, `cognitive_max: 5`, `min_assertions: 1` |
| library (`pkg/**`, `lib/**`) | exported: `params_max: 3`, `cyclomatic_max: 8`, `lines_max: 30`, docstrings required |
| script (`scripts/**`, `examples/**`) | `function_lines_max: 100`, `file_lines_max: 300` |

Gate workflow:

- `just dogfood` — run the structural gate against this repo in gate mode; known findings are green via the committed `pretender-baseline.json`, new ones fail.
- `just dogfood-red <path> <rule>` — remove all ledger entries for one path+rule pair (removal only, additions forbidden) so the now-failing findings print RED; fix the code, not the config.
- The lefthook pre-commit gate runs `cargo run --quiet -p pretender -- check . --mode gate`, so the gate holds on every commit, not just CI.

**Baseline invariant:** a refactor PR that eliminates findings must ship the refreshed `pretender-baseline.json` in the same PR, and the ledger may only *shrink* — never add IDs. Example ID shape:

```
pretender/src/primitive_obsession.rs::analyze::cognitive
```

If a function exceeds a threshold, decompose it into smaller verbatim-behaving helpers rather than raising a limit.

## Submitting a pull request

1. Create a branch from `main`
2. Make your changes
3. Run `just ci` to confirm all gates pass
4. Open a pull request — the CI workflow will run automatically

## Docs

To preview the documentation site locally:

```sh
just book-serve   # requires: cargo install mdbook
```
