//! Integration tests for the `check` subcommand.
//!
//! Exercises the `check` CLI end to end: default/gate/tiered/guidance exit
//! policy, human/JSON/SARIF output shapes, config warnings, `--staged` and
//! `--diff-only` git filtering, exclude globs, binary skipping, min
//! assertions for the test role, data-model rule wiring, and resolution
//! tracking across runs.
//!
//! # Responsibilities
//! Declare the per-topic check-test submodules and the shared harness; each
//! submodule runs the built binary against staged fixtures and pins
//! observable stdout / stderr / exit-code / JSON-envelope behavior.
//!
//! # Rationale
//! Split out of `tests/cli_test.rs` (wave 3 per-command test split) and
//! partitioned into submodules so every file stays within the gate's
//! per-file size limit; test bodies moved verbatim with expect/unwrap calls
//! converted to the sanctioned panic-on-failure forms.

mod common;

// Explicit #[path]: integration-test crate roots resolve child modules
// directory-relative in this toolchain, so the per-topic files under
// tests/check_test/ must be pinned by path.
#[path = "check_test/config_test.rs"]
mod config_test;
#[path = "check_test/data_model_test.rs"]
mod data_model_test;
#[path = "check_test/entry_test.rs"]
mod entry_test;
#[path = "check_test/exported_scope_test.rs"]
mod exported_scope_test;
#[path = "check_test/git_filter_test.rs"]
mod git_filter_test;
#[path = "check_test/human_output_test.rs"]
mod human_output_test;
#[path = "check_test/min_assertions_test.rs"]
mod min_assertions_test;
#[path = "check_test/mock_support_test.rs"]
mod mock_support_test;
#[path = "check_test/modes_test.rs"]
mod modes_test;
#[path = "check_test/resolution_test.rs"]
mod resolution_test;
#[path = "check_test/unwrap_exclusions_test.rs"]
mod unwrap_exclusions_test;
