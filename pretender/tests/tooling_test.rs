//! Integration tests for pretender's tooling surfaces.
//!
//! Exercises the `mutation`, `complexity`, `duration`, `ci generate`, and
//! `plugins` subcommands plus the CLI surface (stub exits, typo
//! suggestions, `--version`, `feedback`, `explain`) end to end.
//!
//! # Responsibilities
//! Declare the per-topic tooling-test submodules; each submodule runs the
//! built binary against fixtures and pins observable stdout / stderr /
//! exit-code / JSON behavior.
//!
//! # Rationale
//! Final wave-3 split child: the mutation/duplication/plugins tests and
//! everything remaining in `tests/cli_test.rs` moved verbatim (with
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms)
//! and partitioned into submodules so every file stays within the gate's
//! per-file size limit; `tests/cli_test.rs` is deleted by this move.

mod common;

// Explicit #[path]: integration-test crate roots resolve child modules
// directory-relative in this toolchain, so the per-topic files under
// tests/tooling_test/ must be pinned by path.
#[path = "tooling_test/cli_surface_test.rs"]
mod cli_surface_test;
#[path = "tooling_test/complexity_languages_test.rs"]
mod complexity_languages_test;
#[path = "tooling_test/complexity_test.rs"]
mod complexity_test;
#[path = "tooling_test/duration_test.rs"]
mod duration_test;
#[path = "tooling_test/mutation_test.rs"]
mod mutation_test;
#[path = "tooling_test/plugins_test.rs"]
mod plugins_test;
