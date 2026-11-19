# Tasks: fix-cli-spec-drift

- [x] 1. Establish ground truth: run `cargo test --test cli_test test_explain_known_metric_prints_doc` and the `test_check_staged_*` family; confirm shipped behavior (GREEN).
- [x] 2. Write delta spec (`specs/cli-and-config/spec.md`, MODIFIED `Check Command` + `Reserved Commands`) mapping to the two drifted scenarios.
- [x] 3. Apply delta to `openspec/specs/cli-and-config/spec.md`.
- [x] 4. Remove DRIFT pins from `.espectacular/cli-and-config/reserved-command-reports-not-implemented.toml` and `reserved-staged-flag-is-rejected.toml`.
- [x] 5. `openspec validate --strict` and `ah check` green.
