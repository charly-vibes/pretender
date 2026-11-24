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

# Golden behavior corpus (5zo.20): lock the analyzer's findings output over
# pretender/tests/fixtures so pure refactors cannot change behavior silently.
# The corpus is copied to a temp dir outside the repo under a neutral name —
# **/fixtures/** is exclude-filtered even for explicit paths, and a temp dir
# discovers no config, so default excludes apply. Paths are made relative by
# scanning from inside the temp dir; the jq filter (governance/golden.jq)
# canonicalizes to a deterministic document (sorted files/units/violations,
# metrics only, no line numbers or envelope metadata) so byte-identity means
# behavior-identity.
golden file="governance/golden.json":
    #!/usr/bin/env bash
    set -euo pipefail
    repo="$PWD"
    cargo build --quiet -p pretender
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    cp -r pretender/tests/fixtures/. "$tmp/corpus"
    cd "$tmp"
    # `check` exits non-zero when the corpus has findings — that is expected
    # here; raw.json is still written and jq below validates it.
    "$repo/target/debug/pretender" check corpus --format json > raw.json || true
    jq -S -f "$repo/governance/golden.jq" raw.json > "$repo/{{file}}"
    echo "golden: wrote {{file}}"

# Verify the committed golden still matches the analyzer's output (exit
# non-zero on any behavior change; regenerate deliberately with `just golden`
# and review the diff).
golden-check file="governance/golden.json":
    #!/usr/bin/env bash
    set -euo pipefail
    repo="$PWD"
    cargo build --quiet -p pretender
    tmp="$(mktemp -d)"
    trap 'rm -rf "$tmp"' EXIT
    cp -r pretender/tests/fixtures/. "$tmp/corpus"
    cd "$tmp"
    # See `golden`: findings exit non-zero by design; jq validates the JSON.
    "$repo/target/debug/pretender" check corpus --format json > raw.json || true
    jq -S -f "$repo/governance/golden.jq" raw.json > fresh.json
    diff -u "$repo/{{file}}" fresh.json
    echo "golden-check: behavior unchanged"

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
    tmp="$(mktemp)"
    trap 'rm -f "$tmp"' EXIT
    # Single filter instance: mutate first, guard the ACTUAL output, then
    # move it into place (review finding: the guard used to measure a
    # hand-copied duplicate of this filter, so drift at the mutation could
    # evade it).
    jq --arg path "$path" --arg rule "$rule" \
        '.finding_ids |= map(select((split("::")[0] != $path) or ((split("::") | last) != $rule)))' \
        "$baseline" > "$tmp"
    after="$(jq '.finding_ids | length' "$tmp")"
    if (( after > before )); then
        echo "dogfood-red: guard violated: filtered set ($after) larger than input ($before); removal only, additions forbidden" >&2
        exit 1
    fi
    cat "$tmp" > "$baseline"
    echo "dogfood-red: removed $((before - after)) finding id(s) for ${path}::${rule} from ${baseline} (${before} -> ${after})"
    if [[ "$baseline" == "pretender-baseline.json" ]]; then
        cargo run --quiet -p pretender -- check .
    fi

# Full CI gate: fmt + type-check + lint + test + structural gate + golden behavior lock
ci: fmt-check type-check lint test dogfood golden-check

# Project-local preflight gate.
# Prefer this over `bd preflight` for now; the current beads preflight is
# Go-specific upstream and does not reflect this Rust workspace.
preflight: ci

# Show project status
status:
    wai status
    bd ready
