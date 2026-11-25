//! Integration tests for `test_report::parse_junit` and
//! `test_report::resolve_testcase_file` (moved from the former in-file
//! `#[cfg(test)]` module, pretender-5zo.15.1).

use pretender::config::TimeUnit;
use pretender::test_report::{parse_junit, resolve_testcase_file, TestStatus, TestTiming};
use std::io::Write;
use std::path::{Path, PathBuf};

/// JUnit XML in a temp file (original `make_junit_xml`, panic-on-error form).
fn junit_xml(content: &str) -> tempfile::NamedTempFile {
    let mut file = tempfile::NamedTempFile::new().unwrap_or_else(|e| panic!("temp file: {e:?}"));
    write!(file, "{content}").unwrap_or_else(|e| panic!("write xml: {e:?}"));
    file
}

fn parse_with_unit(xml: &str, unit: TimeUnit) -> Vec<TestTiming> {
    parse_junit(junit_xml(xml).path(), unit).unwrap_or_else(|e| panic!("parse: {e:?}"))
}

fn parse_secs(xml: &str) -> Vec<TestTiming> {
    parse_with_unit(xml, TimeUnit::Seconds)
}

/// Create `rel` under `dir` (existence is all resolution needs).
fn write_fixture(dir: &Path, rel: &str) -> PathBuf {
    let src = dir.join(rel);
    std::fs::create_dir_all(src.parent().unwrap_or_else(|| panic!("no parent: {rel}")))
        .unwrap_or_else(|e| panic!("mkdir {rel}: {e:?}"));
    std::fs::write(&src, "def test_x(): pass").unwrap_or_else(|e| panic!("write {rel}: {e:?}"));
    src
}

#[test]
fn parses_single_testcase() {
    let timings = parse_secs(
        r#"<?xml version="1.0"?>
        <testsuite name="test_suite" tests="1">
            <testcase classname="test_math" name="test_addition" time="0.015"/>
        </testsuite>"#,
    );
    assert_eq!(timings.len(), 1);
    assert_eq!(timings[0].name, "test_addition");
    assert_eq!(timings[0].classname, "test_math");
    assert_eq!(timings[0].duration_ms, 15); // 0.015 * 1000 = 15
    assert_eq!(timings[0].status, TestStatus::Passed);
}

#[test]
fn parses_multiple_testsuites() {
    let timings = parse_secs(
        r#"<?xml version="1.0"?>
        <testsuites>
            <testsuite name="suite1" tests="1">
                <testcase classname="a" name="t1" time="0.1"/>
            </testsuite>
            <testsuite name="suite2" tests="1">
                <testcase classname="b" name="t2" time="0.2"/>
            </testsuite>
        </testsuites>"#,
    );
    assert_eq!(timings.len(), 2);
    assert_eq!(timings[0].name, "t1");
    assert_eq!(timings[1].name, "t2");
}

#[test]
fn handles_milliseconds_time_unit() {
    let timings = parse_with_unit(
        r#"<?xml version="1.0"?>
        <testsuite name="ts" tests="1">
            <testcase classname="c" name="t" time="150"/>
        </testsuite>"#,
        TimeUnit::Milliseconds,
    );
    assert_eq!(timings[0].duration_ms, 150);
}

#[test]
fn round_half_up_properly() {
    let timings = parse_secs(
        r#"<?xml version="1.0"?>
        <testsuite name="ts" tests="1">
            <testcase classname="c" name="t" time="0.0155"/>
        </testsuite>"#,
    );
    // 0.0155 * 1000 = 15.5 → round-half-up = 16
    assert_eq!(timings[0].duration_ms, 16);
}

#[test]
fn excludes_skipped_testcases() {
    let timings = parse_secs(
        r#"<?xml version="1.0"?>
        <testsuite name="ts" tests="2">
            <testcase classname="c" name="passing" time="0.01"/>
            <testcase classname="c" name="skipped" time="0.02">
                <skipped/>
            </testcase>
        </testsuite>"#,
    );
    assert_eq!(timings.len(), 1);
    assert_eq!(timings[0].name, "passing");
}

#[test]
fn includes_failed_and_errored() {
    let timings = parse_secs(
        r#"<?xml version="1.0"?>
        <testsuite name="ts" tests="3">
            <testcase classname="c" name="fail" time="0.1">
                <failure message="assert failed"/>
            </testcase>
            <testcase classname="c" name="err" time="0.2">
                <error message="exception"/>
            </testcase>
            <testcase classname="c" name="pass" time="0.3"/>
        </testsuite>"#,
    );
    assert_eq!(timings.len(), 3);
    assert_eq!(timings[0].status, TestStatus::Failed);
    assert_eq!(timings[1].status, TestStatus::Errored);
    assert_eq!(timings[2].status, TestStatus::Passed);
}

#[test]
fn skips_testcases_without_time_attribute() {
    let timings = parse_secs(
        r#"<?xml version="1.0"?>
        <testsuite name="ts" tests="2">
            <testcase classname="c" name="with_time" time="0.1"/>
            <testcase classname="c" name="no_time"/>
        </testsuite>"#,
    );
    assert_eq!(timings.len(), 1);
    assert_eq!(timings[0].name, "with_time");
}

#[test]
fn handles_empty_xml_report() {
    let timings = parse_secs(
        r#"<?xml version="1.0"?>
        <testsuite name="empty" tests="0">
        </testsuite>"#,
    );
    let empty: Vec<TestTiming> = Vec::new();
    assert_eq!(timings, empty);
}

#[test]
fn handles_malformed_xml_gracefully() {
    let timings = parse_secs(
        r#"<?xml version="1.0"?>
        <testsuite name="ts" tests="1">
            <testcase classname="c" name="good" time="0.1"/>
            <testcase classname="c" name="bad" time="0.2""
        </testsuite>"#,
    );
    // Should not panic; the parser stops early and returns what it parsed
    assert!(!timings.is_empty());
}

#[test]
fn independent_rerun_entries_evaluated_separately() {
    let timings = parse_secs(
        r#"<?xml version="1.0"?>
        <testsuite name="ts" tests="2">
            <testcase classname="c" name="flaky_test" time="0.5"/>
            <testcase classname="c" name="flaky_test" time="0.3"/>
        </testsuite>"#,
    );
    assert_eq!(timings.len(), 2);
    assert_eq!(timings[0].name, "flaky_test");
    assert_eq!(timings[0].duration_ms, 500);
    assert_eq!(timings[1].duration_ms, 300);
}

#[test]
fn time_attribute_fractional_seconds() {
    let timings = parse_secs(
        r#"<?xml version="1.0"?>
        <testsuite name="ts" tests="1">
            <testcase classname="c" name="slow" time="1.234567"/>
        </testsuite>"#,
    );
    let first = timings
        .first()
        .unwrap_or_else(|| panic!("one timing expected"));
    assert_eq!(first.duration_ms, 1235); // 1.234567 * 1000 = 1234.567 → round-half-up = 1235
}

#[test]
fn file_attribute_used_when_present() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e:?}"));
    let src = write_fixture(dir.path(), "tests/test_foo.py");

    let resolved = resolve_testcase_file(Some(&src), "test_foo", "tests", dir.path());
    assert_eq!(resolved, Some(src));
}

#[test]
fn classname_to_path_fallback_works() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e:?}"));
    write_fixture(dir.path(), "tests/test_math.rs");

    let resolved = resolve_testcase_file(None, "test_math", "tests", dir.path());
    assert_eq!(resolved, Some(PathBuf::from("tests/test_math.rs")));
}

#[test]
fn classname_with_dots_maps_to_path() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e:?}"));
    write_fixture(dir.path(), "tests/com/example/widget_test.py");

    let resolved = resolve_testcase_file(None, "com.example.widget_test", "tests", dir.path());
    assert_eq!(
        resolved,
        Some(PathBuf::from("tests/com/example/widget_test.py"))
    );
}

#[test]
fn classname_unresolved_returns_none() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e:?}"));
    let resolved = resolve_testcase_file(None, "nonexistent.test", "tests", dir.path());
    assert!(resolved.is_none());
}

#[test]
fn file_not_found_falls_back_to_classname() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e:?}"));
    write_fixture(dir.path(), "tests/test_widget.py");

    // file attribute points to nonexistent path, classname should match
    let resolved = resolve_testcase_file(
        Some(Path::new("nonexistent/foo.py")),
        "test_widget",
        "tests",
        dir.path(),
    );
    assert_eq!(resolved, Some(PathBuf::from("tests/test_widget.py")));
}
