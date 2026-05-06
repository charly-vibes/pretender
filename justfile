# pretender — language-agnostic code quality CLI
# https://just.systems

default:
    @just --list

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

# Type-check without building
check:
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

# Generate docs
doc:
    cargo doc --no-deps --open

# Clean build output
clean:
    cargo clean

# Full CI gate: fmt + lint + test
ci: fmt-check lint test

# Show project status
status:
    wai status
    bd ready
