//! Test-duration reporting tests (JUnit XML ingestion via `check --test-report`).
//!
//! # Responsibilities
//! Pin human/JSON/SARIF rendering of test duration findings, the
//! no-threshold negative case, and the `.pretender/last-check.json` cache
//! persistence of test findings.
//!
//! # Rationale
//! Bodies moved verbatim from `tests/cli_test.rs` (wave-3 split) with
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms;
//! the JUnit/temp-dir fixture setup is shared via `duration_setup`.

use crate::common::*;

use std::path::PathBuf;

/// Helper: set up a temp dir with a pretender.toml, a test file, and a JUnit XML report.
fn duration_setup() -> (PathBuf, PathBuf, PathBuf) {
    let (dir, _guard) = write_temp_file(
        "pretender.toml",
        r#"
        [thresholds.unit-test]
        duration_max_ms = 50
        [roles]
        test = { paths = [] }
        unit-test = { paths = [] }
        integration-test = { paths = [] }
        "#,
    );
    // Create the test file so role detection can find it
    let test_file = dir.join("tests/unit/test_math.py");
    std::fs::create_dir_all(
        test_file
            .parent()
            .unwrap_or_else(|| panic!("test file path has no parent")),
    )
    .unwrap_or_else(|e| panic!("create dirs: {e:?}"));
    std::fs::write(&test_file, "def test_addition(): pass")
        .unwrap_or_else(|e| panic!("write test file: {e:?}"));

    let junit_xml = format!(
        r#"<?xml version="1.0"?>
    <testsuite name="ts" tests="1">
        <testcase classname="test_math" name="test_addition" file="{}" time="0.1"/>
    </testsuite>"#,
        test_file.display()
    );
    let (_, report_path) = write_temp_file("test-report.xml", &junit_xml);
    (dir, test_file, report_path)
}

#[test]
fn test_duration_report_human_output() {
    let (dir, _test_file, report_path) = duration_setup();

    let output = pretender_cmd()
        .arg("check")
        .arg("--test-report")
        .arg(&report_path)
        .arg("--format")
        .arg("human")
        .env("NO_COLOR", "1")
        .current_dir(&dir)
        .output()
        .unwrap_or_else(|e| panic!("pretender check: {e:?}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Test duration findings"),
        "expected 'Test duration findings' in stdout: {stdout}"
    );
    assert!(
        stdout.contains("test_addition"),
        "expected test name in stdout: {stdout}"
    );
    assert!(
        stdout.contains("100ms > 50ms"),
        "expected duration violation in stdout: {stdout}"
    );
}

#[test]
fn test_duration_report_json_contains_findings() {
    let (dir, _test_file, report_path) = duration_setup();

    let output = pretender_cmd()
        .arg("check")
        .arg("--test-report")
        .arg(&report_path)
        .arg("--format")
        .arg("json")
        .env("NO_COLOR", "1")
        .current_dir(&dir)
        .output()
        .unwrap_or_else(|e| panic!("pretender check: {e:?}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("test_findings"),
        "expected 'test_findings' in JSON output: {stdout}"
    );
    assert!(
        stdout.contains("test_addition"),
        "expected test name in JSON: {stdout}"
    );
}

#[test]
fn test_duration_report_sarif_contains_findings() {
    let (dir, _test_file, report_path) = duration_setup();

    let output = pretender_cmd()
        .arg("check")
        .arg("--test-report")
        .arg(&report_path)
        .arg("--format")
        .arg("sarif")
        .env("NO_COLOR", "1")
        .current_dir(&dir)
        .output()
        .unwrap_or_else(|e| panic!("pretender check: {e:?}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("pretender/test-duration"),
        "expected 'pretender/test-duration' rule id in SARIF: {stdout}"
    );
    assert!(
        stdout.contains("test_addition"),
        "expected test name in SARIF: {stdout}"
    );
}

#[test]
fn test_duration_no_threshold_no_findings() {
    let (dir, _guard) = write_temp_file(
        "pretender.toml",
        r#"
        [roles]
        test = { paths = [] }
        unit-test = { paths = [] }
        integration-test = { paths = [] }
        "#,
    );
    let test_file = dir.join("tests/unit/test_math.py");
    std::fs::create_dir_all(
        test_file
            .parent()
            .unwrap_or_else(|| panic!("test file path has no parent")),
    )
    .unwrap_or_else(|e| panic!("create dirs: {e:?}"));
    std::fs::write(&test_file, "def test_addition(): pass")
        .unwrap_or_else(|e| panic!("write test file: {e:?}"));

    let junit_xml = format!(
        r#"<?xml version="1.0"?>
    <testsuite name="ts" tests="1">
        <testcase classname="test_math" name="test_addition" file="{}" time="0.1"/>
    </testsuite>"#,
        test_file.display()
    );
    let (_, report_path) = write_temp_file("test-report.xml", &junit_xml);

    let output = pretender_cmd()
        .arg("check")
        .arg("--test-report")
        .arg(&report_path)
        .arg("--format")
        .arg("json")
        .env("NO_COLOR", "1")
        .current_dir(&dir)
        .output()
        .unwrap_or_else(|e| panic!("pretender check: {e:?}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains("test_findings"),
        "expected no test_findings when no threshold configured: {stdout}"
    );
}

#[test]
fn test_duration_cache_persisted() {
    let (dir, _test_file, report_path) = duration_setup();

    let output = pretender_cmd()
        .arg("check")
        .arg("--test-report")
        .arg(&report_path)
        .arg("--format")
        .arg("json")
        // raw build: tests exit policy (gate default) via its own config dir,
        // helper flags would shadow it (pretender-1te)
        .arg("--mode")
        .arg("tiered")
        .arg("--advisory-until")
        .arg("2099-12-31")
        .env("NO_COLOR", "1")
        .current_dir(&dir)
        .output()
        .unwrap_or_else(|e| panic!("pretender check: {e:?}"));
    assert!(output.status.success());

    // Check that the cache file contains test_findings
    let cache_path = dir.join(".pretender/last-check.json");
    assert!(cache_path.exists(), "cache file should exist");
    let cache_content =
        std::fs::read_to_string(&cache_path).unwrap_or_else(|e| panic!("read cache: {e:?}"));
    assert!(
        cache_content.contains("test_findings"),
        "expected test_findings in cache: {cache_content}"
    );
}
