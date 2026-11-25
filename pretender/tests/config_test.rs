//! Config schema integration tests.
//!
//! # Responsibilities
//! Pin `pretender.toml` parsing through the public config API: the full
//! schema (including unknown-key tolerance), the advisory-until lease
//! field, data-model threshold defaults and role-table overrides, and
//! the domain-pattern list.
//!
//! # Rationale
//! Moved verbatim from the `#[cfg(test)]` module in `src/config.rs`
//! (wave 4 ratchet); expect/unwrap calls converted to the sanctioned
//! panic-on-failure forms. The one oversized schema test was split into
//! two functions over the same source text (same parse, same assertions).

// Explicit #[path]: integration-test crate roots resolve child modules
// directory-relative in this toolchain (see check_test.rs).
#[path = "config_test/defaults.rs"]
mod defaults;
#[path = "config_test/lease.rs"]
mod lease;
#[path = "config_test/schema.rs"]
mod schema;
#[path = "config_test/validation.rs"]
mod validation;
