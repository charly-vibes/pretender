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
