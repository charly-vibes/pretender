---
tags: [pipeline-run:tdd-ro5u-2026-05-12-pretender-81c-cli-check-command, pipeline-step:verify-close]
---

# pretender-81c: Complete check command MVP slice

## Current state
- `pretender check [paths...]` works with `--format human|json` for python files.
- Per-role thresholds applied; violations included in report.
- Tests in `pretender/tests/cli_test.rs` cover human + json paths.

## Desired end state
- `--output <path>` writes formatted report to a file (defaults to stdout).
- Human output is colored on tty; respects `NO_COLOR` env var; suppressed when writing to file.
- Exit code is non-zero (1) when any violation is found; 0 otherwise.
- File processing parallelized with rayon (deterministic ordering).

## Out of scope (deferred to other issues)
- `--staged`, `--diff-only`, `--diff-base` → pretender-a80 (git2 integration)
- SARIF format → pretender-t2m
- JUnit, markdown, html formats → pretender-3b5 (report command)
- guidance/tiered/gate mode flag → pretender-6aw (this just emits the default gate-style exit code)

## Phases (TDD)
1. **Exit code on violations** — RED: fixture with a violation → exit 1; clean fixture → exit 0. GREEN: track violation count and return ExitCode::FAILURE.
2. **--output flag** — RED: test that report is written to provided file path and stdout is empty. GREEN: route writes through a `Box<dyn Write>` sink.
3. **Colored human output** — RED: human output to file (or `NO_COLOR=1`) is plain; tty path uses ANSI. Use `owo-colors` (already MIT, light). GREEN: gate colorization on tty + NO_COLOR.
4. **Rayon parallelization** — RED-ish: assert ordering stable across runs on a small dir fixture. GREEN: replace iter with par_iter, collect into Vec, sort by path before emission.

## Risks
- rayon could destabilize ordering — fix by post-sort.
- color in test snapshots — disable via `NO_COLOR=1` in tests or always when stdout is not tty.
