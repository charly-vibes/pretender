//! JUnit XML report parsing for test-duration checks.
//!
//! Parses JUnit XML [`<testsuite>` / `<testcase>`] reports and converts
//! timing data to milliseconds. Supports `time` in seconds (XSD-conformant)
//! or milliseconds, with round-half-up conversion to u32.
//!
//! # Responsibilities
//! The shared data types ([`TestTiming`], [`TestStatus`],
//! [`TestDurationFinding`]) and submodule wiring: XML parsing in `junit`,
//! threshold evaluation in `evaluate`, time conversion in `duration`, and
//! testcase file resolution in `resolve`.
//!
//! # Rationale
//! Split from the single 870-line `test_report.rs` (pretender-5zo.15.1)
//! along the parse / convert / evaluate / resolve seams so every file and
//! function stays within the structural limits; bodies moved verbatim
//! except where a function had to split further, with the in-file unit
//! tests promoted to the `tests/` integration targets.

#![allow(dead_code)]

pub use evaluate::evaluate_duration;
pub use junit::parse_junit;
pub use resolve::resolve_testcase_file;

mod duration;
mod evaluate;
mod junit;
mod resolve;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A finding emitted when a test exceeds its duration threshold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestDurationFinding {
    /// Test name from the JUnit report.
    pub test_name: String,
    /// Resolved source file path, if available.
    pub file: Option<PathBuf>,
    /// Role applied to this test (unit-test, integration-test, or test).
    pub role: String,
    /// Observed duration in milliseconds.
    pub observed_ms: u32,
    /// Threshold that was exceeded, in milliseconds.
    pub threshold_ms: u32,
}

/// Timing data extracted for one JUnit testcase.
#[derive(Debug, Clone, PartialEq)]
pub struct TestTiming {
    /// Test name from the `name` attribute.
    pub name: String,
    /// Class name from the `classname` attribute (may be empty).
    pub classname: String,
    /// Resolved source file path, if available.
    pub file: Option<PathBuf>,
    /// Elapsed time in milliseconds (round-half-up from the `time` attribute).
    pub duration_ms: u32,
    /// Execution status.
    pub status: TestStatus,
}

/// Execution status of a test case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    /// Test passed (no `<skipped>`, `<failure>`, or `<error>` child).
    Passed,
    /// Test was skipped (has `<skipped>` child).
    Skipped,
    /// Test failed (has `<failure>` child).
    Failed,
    /// Test errored (has `<error>` child).
    Errored,
}
