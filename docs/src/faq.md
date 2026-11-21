# FAQ

Answers to the questions that come up most when adopting pretender. For
thresholds and every configuration key, see the [Configuration
reference](configuration.md); for a day-one walkthrough, see
[Getting Started](getting-started.md).

## Do I need this if I already use clippy / eslint / rubocop?

Those linters catch style and lint-level issues. pretender gates *structural*
quality: cyclomatic, cognitive and abc complexity per function, structural
clones, and minimum assertions per test — each against role-aware thresholds
(`pretender explain <metric>` prints them). It complements linters and type
checkers; it does not replace them.

## What does pretender write or modify?

- `pretender.toml` in the repo root (written by `pretender init`)
- Optionally `.git/hooks/pre-commit` (or `pre-push`) — a native shim installed
  by `pretender hooks install`, removed by `pretender hooks uninstall`
- `.github/workflows/pretender.yml` if you run `pretender ci generate github`
- A cache directory under `XDG_CACHE_HOME` (default `~/.cache/pretender`)

It never edits your source files.

## How do I run it in CI?

`pretender ci generate github` writes a ready-to-commit workflow that runs the
check on every pull request. Commit the file; the gate then runs on GitHub's
runners with no extra setup.

## Why does it fail on a clean checkout of an existing repo?

Because the gate counts every function in the scanned paths, not only new
code. Legacy hot spots are expected: record them once as the known-findings
baseline (`--adopt` maintains `pretender-baseline.json`), after which only
*new* violations fail the gate. Scope any check to the PR diff with
`--staged` or `--diff-only`.

## What is the advisory lease (`--advisory-until`)?

`guidance` and `tiered` modes report findings as advisories instead of hard
failures, and require a dated lease (`advisory_until` in `pretender.toml` or
`--advisory-until YYYY-MM-DD`). An expired or missing lease fails closed —
renewing it is a visible, reviewable diff, so a soft gate cannot silently
stay soft.

## How do I disable or soften the gate?

- **Soften:** switch `mode` to `guidance` or `tiered` in `pretender.toml`
  (with a current advisory lease); advisories print but exit 0.
- **Disable:** remove the hook (`pretender hooks uninstall`) and the generated
  CI workflow, or stop calling `pretender check` in CI.

## What about plugins?

The `pretender plugins` command exists but is [not yet implemented]; language
and metric plugins are on the roadmap. See [Writing plugins](plugins.md) for
the intended surface and [Status](status.md) for what ships today.
