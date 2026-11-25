//! Check-command pipeline.
//!
//! Implements the `check` subcommand end to end: config and lease
//! preparation, file collection, per-file analysis, coupling and
//! test-duration passes, gate-baseline handling, report writing, and the
//! exit-code decision.
//!
//! # Responsibilities
//! Own everything `pretender check` does between argument parsing and the
//! returned exit code. Report rendering lives in `crate::report`; the
//! report data model stays at the crate root so all writers share it.
//!
//! # Rationale
//! Extracted verbatim from main.rs (pretender-5zo.2.2) so the bin crate
//! root shrinks below the structural gate's file-size limit; long stage
//! functions were split into verbatim-behaving helpers while moving so
//! every function lands within the function-size and complexity limits.
pub(crate) mod analyze;
pub(crate) mod collect;
pub(crate) mod gate;
pub(crate) mod run;
pub(crate) mod scan;
pub(crate) mod unit;

pub(crate) use collect::collect_input_files;
