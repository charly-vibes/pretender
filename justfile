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

# Per-ticket review independence (pretender-5zo.22).
#
#   just wire-reviewer <ticket> <reviewer-bead-id>
#
# Creates the reviewer bead <reviewer-bead-id> (reused if it already exists)
# with assignee "wave-reviewer" — asserted different from the ticket's
# implementer — then installs a bead-type gate so <ticket> cannot close while
# the reviewer bead is open: `bd close <ticket>` is refused and <ticket> is
# hidden from `bd ready` until the gate resolves. Closing the reviewer bead
# does NOT auto-resolve the gate; run `bd gate check --type=bead` afterwards,
# then close <ticket>. `bd close <ticket> --force` bypasses the gate and is
# reserved for genuine emergencies.
# Example: just wire-reviewer pretender-5zo.29 pretender-5zo.29.rev
wire-reviewer ticket reviewer_id:
    #!/usr/bin/env bash
    set -euo pipefail
    reviewer_assignee="wave-reviewer"
    bd show "{{ticket}}" >/dev/null 2>&1 || { echo "wire-reviewer: ticket {{ticket}} not found" >&2; exit 1; }
    implementer="$(bd show "{{ticket}}" --json | jq -r '.[0].assignee // empty')"
    if [[ -n "$implementer" && "$implementer" == "$reviewer_assignee" ]]; then
        echo "wire-reviewer: refusing: '$reviewer_assignee' is also the implementer of {{ticket}}; review must be independent" >&2
        exit 1
    fi
    if bd show "{{reviewer_id}}" >/dev/null 2>&1; then
        echo "wire-reviewer: reviewer bead {{reviewer_id}} already exists; reusing"
    else
        bd create --id "{{reviewer_id}}" --silent --assignee "$reviewer_assignee" --title "Wave review: {{ticket}}" --description "Review-independence guard for {{ticket}}: to be closed by a reviewer (assignee: $reviewer_assignee) distinct from the implementer${implementer:+ ($implementer)}. {{ticket}} is gated on this bead and cannot close while it is open."
        echo "wire-reviewer: created reviewer bead {{reviewer_id}} (assignee: $reviewer_assignee)"
    fi
    if bd gate list --json | jq -e --arg r "{{reviewer_id}}" '(. // [])[] | select(.await_type == "bead" and .await_id == $r)' >/dev/null; then
        echo "wire-reviewer: bead gate on {{reviewer_id}} is already open; nothing to do"
        exit 0
    fi
    bd gate create --type=bead --blocks "{{ticket}}" --await-id "{{reviewer_id}}" --title "Gate: wave review of {{ticket}}" --reason "review independence: {{ticket}} cannot close while reviewer bead {{reviewer_id}} is open"
    echo "wire-reviewer: {{ticket}} is gated on {{reviewer_id}}; bd close {{ticket}} now refuses until the reviewer bead is closed and 'bd gate check --type=bead' resolves the gate"
