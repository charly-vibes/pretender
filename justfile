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

# Validate spec-test correspondence (requires ah)
validate:
    ah check

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Type-check without building (named to avoid confusion with `pretender check`)
type-check:
    cargo check

# Lint (deny warnings, all targets)
lint:
    cargo clippy --all-targets -- -D warnings

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

# Dogfood: run pretender's structural gate against this repo's own source.
# Gate mode + the committed pretender-baseline.json means known findings stay
# green and only genuinely new ones fail.
dogfood:
    cargo run --quiet -p pretender -- check .

# Un-grandfather baseline findings for one path+rule pair to produce a real RED.
# Ledger model — removal only, additions forbidden (monotonicity gate).
# With the default baseline this filters the ledger in place, then runs the gate
# so the now-failing findings print. With an explicit baseline argument it only
# filters and reports the removal count, so it is testable on temp copies.
dogfood-red path rule baseline="pretender-baseline.json":
    #!/usr/bin/env bash
    set -euo pipefail
    path='{{path}}'
    rule='{{rule}}'
    baseline='{{baseline}}'
    if [[ ! -f "$baseline" ]]; then
        echo "dogfood-red: baseline file not found: $baseline" >&2
        exit 1
    fi
    before="$(jq '.finding_ids | length' "$baseline")"
    after="$(jq --arg path "$path" --arg rule "$rule" \
        '[.finding_ids[] | select((split("::")[0] != $path) or ((split("::") | last) != $rule))] | length' \
        "$baseline")"
    if (( after > before )); then
        echo "dogfood-red: guard violated: filtered set ($after) larger than input ($before); removal only, additions forbidden" >&2
        exit 1
    fi
    tmp="$(mktemp)"
    trap 'rm -f "$tmp"' EXIT
    jq --arg path "$path" --arg rule "$rule" \
        '.finding_ids |= map(select((split("::")[0] != $path) or ((split("::") | last) != $rule)))' \
        "$baseline" > "$tmp"
    cat "$tmp" > "$baseline"
    echo "dogfood-red: removed $((before - after)) finding id(s) for ${path}::${rule} from ${baseline} (${before} -> ${after})"
    if [[ "$baseline" == "pretender-baseline.json" ]]; then
        cargo run --quiet -p pretender -- check .
    fi

# Full CI gate: fmt + type-check + lint + test + structural gate
ci: fmt-check type-check lint test dogfood

# Project-local preflight gate.
# Prefer this over `bd preflight` for now; the current beads preflight is
# Go-specific upstream and does not reflect this Rust workspace.
preflight: ci

# Show project status
status:
    wai status
    bd ready
