# Change: Adopt genesis

## Why

pretender is the suite's code-quality gate but lacks the shared envelope
format, self-healing error suggestions, and the feedback subcommand. Adopting
genesis closes those gaps: structured `--json` output through the shared
envelope, typed `Suggestion` footers on errors, and a `feedback` subcommand
wrapping `genesis::feedback` for filing issues upstream.

## What Changes

- Add `genesis` git dependency (pinned by tag `v0.1.0`) to `Cargo.toml`.
- Route `--json` output through `genesis::envelope` (check, complexity,
  duplication, report, mutation results).
- Adopt `genesis::suggestions` for typo detection and fix-footers on
  pretender's command surface (`check`/`complexity`/`report`/`doctor`).
- Add a `pretender feedback [KIND]` subcommand wrapping `genesis::feedback`.
  pretender owns the command surface; genesis owns the machinery.
- Keep all domain logic (metric plugins, AST clone detection, mutation
  wrappers, threshold computation). The genesis boundary rule protects this.

## Impact

- Affected specs: `pretender-cli-core` (MODIFIED — envelope + suggestions +
  feedback).
- Affected code: `Cargo.toml`, `src/main.rs` (new `Feedback` variant + error
  footer), `json.rs` (envelope wrapping).
- Blocked by: genesis tagging `v0.1.0` (envelope/suggestions/feedback stable).
- No user-visible behavior change except `--json` envelopes are now identical
  in shape to wai/dont/espectacular/testaruda.