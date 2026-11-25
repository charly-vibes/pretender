//! Integration tests for the `init`, `doctor`, and `hooks` subcommands.
//!
//! # Responsibilities
//! Pin `init` config writing (non-interactive mode overrides, interactive
//! hook + workflow install) and its foreign-block neutrality, `doctor`'s
//! check/skip reporting in human and JSON envelopes (config, hook
//! installed/executable, Hook location under a redirected core.hooksPath),
//! and `hooks` install/uninstall lifecycle (managed-shim idempotency,
//! foreign-hook refusal, repo-root placement, outside-repo failure).
//!
//! # Rationale
//! Split out of `tests/cli_test.rs` (wave 3 per-command test split) and
//! partitioned into submodules so every file stays within the gate's
//! per-file size limit; test bodies moved verbatim with expect/unwrap calls
//! converted to the sanctioned panic-on-failure forms.

mod common;

// Explicit #[path]: integration-test crate roots resolve child modules
// directory-relative in this toolchain, so the per-topic files under
// tests/init_test/ must be pinned by path.
#[path = "init_test/doctor_test.rs"]
mod doctor_test;
#[path = "init_test/hooks_test.rs"]
mod hooks_test;
#[path = "init_test/init_test.rs"]
mod init_test;
