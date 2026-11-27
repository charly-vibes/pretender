//! `mock_count` test-support-path regressions (pretender-jg9y gap 3).
//!
//! # Responsibilities
//! Pin that shared test-support helpers (`tests/support/*`) stop firing
//! `mock_count`, while the same mock-sprawl source staged as a leaf
//! `*_test.rs` still fires.
//!
//! # Rationale
//! Support modules wire the system under test for the real test binaries;
//! their `.expect(...)` plumbing is not mock usage, and counting it makes
//! the gate noisy where it should stay sharp on leaf tests.

use crate::common::*;
use std::path::Path;

fn fixture_source(name: &str) -> String {
    std::fs::read_to_string(source_fixture(name))
        .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

fn write(path: &Path, content: &str) -> std::path::PathBuf {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("mkdir {}: {e:?}", path.display()));
    }
    std::fs::write(path, content).unwrap_or_else(|e| panic!("write {}: {e:?}", path.display()));
    path.to_path_buf()
}

/// Run `check --mode gate --format json` in `dir` and parse the envelope.
fn run_json(dir: &Path, extra: &[&str]) -> serde_json::Value {
    let output = pretender_cmd()
        .arg("check")
        .arg("--mode")
        .arg("gate")
        .arg("--format")
        .arg("json")
        .args(extra)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("JSON envelope: {e}\nstdout: {stdout}"))
}

/// The mock_count violations reported anywhere in the envelope, as
/// `path actual` strings.
fn mock_findings(envelope: &serde_json::Value) -> Vec<(String, f64)> {
    metric_findings(envelope, "mock_count")
}

#[test]
fn support_helper_path_does_not_fire_mock_count() {
    let dir = tempdir();
    write(
        &dir.join("tests/support/helper.rs"),
        &fixture_source("rust_mock_support_helper.rs"),
    );

    let envelope = run_json(&dir, &[]);
    let findings = mock_findings(&envelope);
    assert!(
        findings.is_empty(),
        "tests/support/ helpers must not fire mock_count; findings: {findings:?}"
    );
}

#[test]
fn leaf_test_mock_sprawl_still_fires() {
    let dir = tempdir();
    // Same source, leaf-test spelling: the exclusion must stay scoped to
    // support paths, so genuine leaf-test sprawl keeps firing.
    write(
        &dir.join("tests/helper_test.rs"),
        &fixture_source("rust_mock_support_helper.rs"),
    );

    let envelope = run_json(&dir, &[]);
    let findings = mock_findings(&envelope);
    assert!(
        !findings.is_empty(),
        "leaf *_test.rs mock sprawl must still fire mock_count"
    );
}
