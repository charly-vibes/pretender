# pretender — language-agnostic code quality CLI
# https://just.systems

default:
    @just --list

# Install pretender binary to ~/.cargo/bin
install:
    cargo install --path pretender --locked

# Build (debug)
build:
    cargo build

# Build (release)
release:
    cargo build --release

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Type-check without building (named to avoid confusion with `pretender check`)
type-check:
    cargo check

# Lint (deny warnings)
lint:
    cargo clippy -- -D warnings

# Format source files
fmt:
    cargo fmt

# Format check (CI)
fmt-check:
    cargo fmt -- --check

# Generate Rust API docs
doc:
    cargo doc --no-deps --open

# Build user-facing docs (requires: cargo install mdbook)
book:
    mdbook build

# Serve docs locally with live reload (requires: cargo install mdbook)
book-serve:
    mdbook serve --open

# Clean build output
clean:
    cargo clean

# Full CI gate: fmt + type-check + lint + test
ci: fmt-check type-check lint test

# Project-local preflight gate.
# Prefer this over `bd preflight` for now; the current beads preflight is
# Go-specific upstream and does not reflect this Rust workspace.
preflight: ci

# Show project status
status:
    wai status
    bd ready
