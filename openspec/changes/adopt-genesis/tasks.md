## 1. Dependency
- [ ] 1.1 Add `genesis = { git = "https://github.com/charly-vibes/genesis", tag = "v0.1.0" }` to `Cargo.toml`.
- [ ] 1.2 Verify build with envelope/suggestions/feedback modules stable.

## 2. Adopt shared envelope
- [ ] 2.1 Route `check`/`complexity`/`duplication`/`mutation`/`report` `--json` through `genesis::envelope`.
- [ ] 2.2 Test: top-level keys match the shared shape.

## 3. Adopt suggestions
- [ ] 3.1 Register pretender's command list with `genesis::suggestions::SuggestionEngine`.
- [ ] 3.2 Wire `main.rs` error sink to emit `genesis::suggestions` fix-footers.
- [ ] 3.3 Regression: `pretender complxity` (typo) prints "Did you mean 'complexity'?".

## 4. Add `feedback` subcommand (wraps `genesis::feedback`)
- [ ] 4.1 Add `Feedback` variant to the `Commands` enum with `KIND` + flags (per agent-issue-reporting playbook §2).
- [ ] 4.2 Read pretender's error scratch (`$XDG_CACHE_HOME/pretender/errors.jsonl`) for `--from-last-error`; never shadow the real error.
- [ ] 4.3 Default target repo = pretender's `Cargo.toml` `repository`; labels from playbook §8.
- [ ] 4.4 Wire the error-footer hook: non-zero exits with no `genesis::suggestions::Fix` print `Feedback: pretender feedback bug --from-last-error`.
- [ ] 4.5 Regression: `pretender feedback bug --dry-run` prints body + exact `gh` line; redactor strips a `https://<pat>@…` remote.

## 5. Clean up
- [ ] 5.1 Remove dead local code; `cargo clippy -- -D warnings` clean.
- [ ] 5.2 Verify tool-craft (genesis `.wai` research) Appendix A.3 pretender row; file a charly-monorepo ticket if inaccurate.