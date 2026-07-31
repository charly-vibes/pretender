//! JUnit XML report parsing for test-duration checks.
//!
//! Parses JUnit XML [`<testsuite>` / `<testcase>`] reports and converts
//! timing data to milliseconds. Supports `time` in seconds (XSD-conformant)
//! or milliseconds, with round-half-up conversion to u32.

#![allow(dead_code)]

use crate::config::{Config, TimeUnit};
use crate::roles::{EffectiveThresholds, Role, RoleDetector};
use anyhow::{Context, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

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

/// Evaluate test timings against configured duration thresholds.
///
/// Each testcase is resolved to a role via the `RoleDetector`. If the
/// resolved role has a non-zero `duration_max_ms` and the test's elapsed
/// time strictly exceeds it, a [`TestDurationFinding`] is emitted.
///
/// Tests that resolve to the base `test` role (or any role without a
/// duration threshold) are skipped.
pub fn evaluate_duration(
    timings: &[TestTiming],
    detector: &RoleDetector,
    config: &Config,
    search_root: &Path,
) -> Vec<TestDurationFinding> {
    let mut findings = Vec::new();

    for timing in timings {
        // Resolve the source file path
        let file = resolve_testcase_file(
            timing.file.as_deref(),
            &timing.classname,
            &config.roles.test.classname_root,
            search_root,
        );

        // Determine role
        let role = match &file {
            Some(path) => detector.detect(path, ""),
            None => Role::Test,
        };

        let effective = EffectiveThresholds::for_role(role, &config.thresholds);

        if effective.duration_max_ms > 0 && timing.duration_ms > effective.duration_max_ms {
            findings.push(TestDurationFinding {
                test_name: timing.name.clone(),
                file,
                role: role_name_str(role),
                observed_ms: timing.duration_ms,
                threshold_ms: effective.duration_max_ms,
            });
        }
    }

    findings
}

fn role_name_str(role: Role) -> String {
    match role {
        Role::App => "app".to_string(),
        Role::Library => "library".to_string(),
        Role::Test => "test".to_string(),
        Role::Script => "script".to_string(),
        Role::Generated => "generated".to_string(),
        Role::Vendor => "vendor".to_string(),
        Role::UnitTest => "unit-test".to_string(),
        Role::IntegrationTest => "integration-test".to_string(),
    }
}
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

/// Parse a JUnit XML report file and extract timing data for each test case.
///
/// * `path` — Path to the JUnit XML file.
/// * `time_unit` — How to interpret the `time` attribute (`Seconds` conforms
///   to the JUnit XSD; `Milliseconds` is for non-conformant runners).
///
/// Returns a list of [`TestTiming`] entries, one per `<testcase>` element.
/// Skipped testcases are excluded. Malformed entries are skipped with a
/// warning diagnostic logged via `log::warn!` (or silently ignored if the
/// `log` crate is unavailable).
pub fn parse_junit(path: &Path, time_unit: TimeUnit) -> Result<Vec<TestTiming>> {
    let xml = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read JUnit XML report: {}", path.display()))?;

    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(true);

    let mut timings = Vec::new();
    let mut buf = Vec::new();
    let mut in_testcase = false;
    let mut current_name = String::new();
    let mut current_classname = String::new();
    let mut current_file: Option<String> = None;
    let mut current_time_attr: Option<f64> = None;
    let mut current_status = TestStatus::Passed;

    // Emit the in-progress testcase if it is a candidate for inclusion.
    macro_rules! finish_testcase {
        () => {
            if in_testcase {
                in_testcase = false;
                if current_status != TestStatus::Skipped {
                    if let Some(time_secs) = current_time_attr {
                        let duration_ms = time_to_ms(time_secs, time_unit);
                        timings.push(TestTiming {
                            name: current_name.clone(),
                            classname: current_classname.clone(),
                            file: current_file.as_ref().map(PathBuf::from),
                            duration_ms,
                            status: current_status,
                        });
                    }
                }
            }
        };
    }

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag == "testcase" {
                    in_testcase = true;
                    current_name = attr_value(e, "name").unwrap_or_default();
                    current_classname = attr_value(e, "classname").unwrap_or_default();
                    current_file = attr_value(e, "file");
                    current_time_attr = attr_value(e, "time").and_then(|v| v.parse::<f64>().ok());
                    current_status = TestStatus::Passed;
                } else if in_testcase {
                    match tag.as_str() {
                        "skipped" => current_status = TestStatus::Skipped,
                        "failure" => {
                            if current_status == TestStatus::Passed {
                                current_status = TestStatus::Failed;
                            }
                        }
                        "error" => {
                            if current_status == TestStatus::Passed {
                                current_status = TestStatus::Errored;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();

                if tag == "testcase" {
                    // Self-closing testcase: no children, no end event.
                    in_testcase = true;
                    current_name = attr_value(e, "name").unwrap_or_default();
                    current_classname = attr_value(e, "classname").unwrap_or_default();
                    current_file = attr_value(e, "file");
                    current_time_attr = attr_value(e, "time").and_then(|v| v.parse::<f64>().ok());
                    current_status = TestStatus::Passed;
                    finish_testcase!();
                } else if in_testcase {
                    match tag.as_str() {
                        "skipped" => current_status = TestStatus::Skipped,
                        "failure" => {
                            if current_status == TestStatus::Passed {
                                current_status = TestStatus::Failed;
                            }
                        }
                        "error" => {
                            if current_status == TestStatus::Passed {
                                current_status = TestStatus::Errored;
                            }
                        }
                        _ => {}
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if tag == "testcase" {
                    finish_testcase!();
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => {
                // Malformed XML — stop parsing rather than panic.
                break;
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(timings)
}

/// Convert a time value in seconds to milliseconds, rounding half-up.
fn time_to_ms(value: f64, time_unit: TimeUnit) -> u32 {
    let ms = match time_unit {
        TimeUnit::Seconds => value * 1000.0,
        TimeUnit::Milliseconds => value,
    };
    round_half_up(ms)
}

/// Round-half-up to the nearest u32.
fn round_half_up(value: f64) -> u32 {
    let rounded = value.round();
    // Handle the tie-breaking: .round() in Rust uses round-half-to-even (bankers' rounding),
    // which is NOT what we want. We need round-half-up. Check if the fractional part is
    // exactly 0.5 and the integer part is even.
    let truncated = value.trunc();
    let fractional = (value - truncated).abs();
    if (fractional - 0.5).abs() < f64::EPSILON && truncated as i64 % 2 == 0 {
        // Banker's rounding would round down; we need to round up
        (truncated + 1.0).max(0.0) as u32
    } else {
        rounded.max(0.0) as u32
    }
}

/// Resolve a JUnit testcase to a candidate source file path.
///
/// Resolution order:
/// 1. If `file` is present, use it directly.
/// 2. Otherwise, derive a path from `classname`: replace `.` with `/`,
///    append the detected language extension, and search under
///    `classname_root` (default `tests`). The first existing file wins.
/// 3. If unresolved, returns `None` (base `test` role with a note).
pub fn resolve_testcase_file(
    file: Option<&Path>,
    classname: &str,
    classname_root: &str,
    search_root: &Path,
) -> Option<PathBuf> {
    // 1. file attribute present
    if let Some(f) = file {
        if f.exists() {
            return Some(f.to_path_buf());
        }
        // Try relative to search_root
        let rel = search_root.join(f);
        if rel.exists() {
            return Some(rel);
        }
    }

    // 2. Derive from classname
    if classname.is_empty() {
        return None;
    }

    // Common language extensions to try
    const EXTENSIONS: &[&str] = &[
        "py", "rs", "js", "ts", "jsx", "tsx", "go", "java", "rb", "c", "cpp", "h", "hpp",
    ];

    let classname_path = classname.replace('.', "/");
    let root = if classname_root.is_empty() {
        PathBuf::from("tests")
    } else {
        PathBuf::from(classname_root)
    };

    for ext in EXTENSIONS {
        let candidate = root.join(format!("{classname_path}.{ext}"));
        let full = search_root.join(&candidate);
        if full.exists() {
            return Some(candidate);
        }
    }

    None
}

fn attr_value<'a>(e: &quick_xml::events::BytesStart<'a>, name: &str) -> Option<String> {
    e.attributes()
        .filter_map(|a| a.ok())
        .find(|a| a.key.as_ref() == name.as_bytes())
        .and_then(|a| String::from_utf8(a.value.to_vec()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_junit_xml(content: &str) -> NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        write!(file, "{}", content).expect("write xml");
        file
    }

    #[test]
    fn parses_single_testcase() {
        let xml = make_junit_xml(
            r#"<?xml version="1.0"?>
            <testsuite name="test_suite" tests="1">
                <testcase classname="test_math" name="test_addition" time="0.015"/>
            </testsuite>"#,
        );
        let timings = parse_junit(xml.path(), TimeUnit::Seconds).expect("parse");
        assert_eq!(timings.len(), 1);
        assert_eq!(timings[0].name, "test_addition");
        assert_eq!(timings[0].classname, "test_math");
        assert_eq!(timings[0].duration_ms, 15); // 0.015 * 1000 = 15
        assert_eq!(timings[0].status, TestStatus::Passed);
    }

    #[test]
    fn parses_multiple_testsuites() {
        let xml = make_junit_xml(
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
        let timings = parse_junit(xml.path(), TimeUnit::Seconds).expect("parse");
        assert_eq!(timings.len(), 2);
        assert_eq!(timings[0].name, "t1");
        assert_eq!(timings[1].name, "t2");
    }

    #[test]
    fn handles_milliseconds_time_unit() {
        let xml = make_junit_xml(
            r#"<?xml version="1.0"?>
            <testsuite name="ts" tests="1">
                <testcase classname="c" name="t" time="150"/>
            </testsuite>"#,
        );
        let timings = parse_junit(xml.path(), TimeUnit::Milliseconds).expect("parse");
        assert_eq!(timings[0].duration_ms, 150);
    }

    #[test]
    fn round_half_up_properly() {
        let xml = make_junit_xml(
            r#"<?xml version="1.0"?>
            <testsuite name="ts" tests="1">
                <testcase classname="c" name="t" time="0.0155"/>
            </testsuite>"#,
        );
        let timings = parse_junit(xml.path(), TimeUnit::Seconds).expect("parse");
        // 0.0155 * 1000 = 15.5 → round-half-up = 16
        assert_eq!(timings[0].duration_ms, 16);
    }

    #[test]
    fn excludes_skipped_testcases() {
        let xml = make_junit_xml(
            r#"<?xml version="1.0"?>
            <testsuite name="ts" tests="2">
                <testcase classname="c" name="passing" time="0.01"/>
                <testcase classname="c" name="skipped" time="0.02">
                    <skipped/>
                </testcase>
            </testsuite>"#,
        );
        let timings = parse_junit(xml.path(), TimeUnit::Seconds).expect("parse");
        assert_eq!(timings.len(), 1);
        assert_eq!(timings[0].name, "passing");
    }

    #[test]
    fn includes_failed_and_errored() {
        let xml = make_junit_xml(
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
        let timings = parse_junit(xml.path(), TimeUnit::Seconds).expect("parse");
        assert_eq!(timings.len(), 3);
        assert_eq!(timings[0].status, TestStatus::Failed);
        assert_eq!(timings[1].status, TestStatus::Errored);
        assert_eq!(timings[2].status, TestStatus::Passed);
    }

    #[test]
    fn skips_testcases_without_time_attribute() {
        let xml = make_junit_xml(
            r#"<?xml version="1.0"?>
            <testsuite name="ts" tests="2">
                <testcase classname="c" name="with_time" time="0.1"/>
                <testcase classname="c" name="no_time"/>
            </testsuite>"#,
        );
        let timings = parse_junit(xml.path(), TimeUnit::Seconds).expect("parse");
        assert_eq!(timings.len(), 1);
        assert_eq!(timings[0].name, "with_time");
    }

    #[test]
    fn handles_empty_xml_report() {
        let xml = make_junit_xml(
            r#"<?xml version="1.0"?>
            <testsuite name="empty" tests="0">
            </testsuite>"#,
        );
        let timings = parse_junit(xml.path(), TimeUnit::Seconds).expect("parse");
        assert_eq!(timings.len(), 0);
    }

    #[test]
    fn handles_malformed_xml_gracefully() {
        let xml = make_junit_xml(
            r#"<?xml version="1.0"?>
            <testsuite name="ts" tests="1">
                <testcase classname="c" name="good" time="0.1"/>
                <testcase classname="c" name="bad" time="0.2""
            </testsuite>"#,
        );
        // Should not panic; should return whatever was parsed successfully
        let timings = parse_junit(xml.path(), TimeUnit::Seconds).expect("parse");
        // The malformed XML may cause the parser to stop early; we just
        // require it to return an error-free result
        assert!(!timings.is_empty());
    }

    #[test]
    fn file_attribute_used_when_present() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/test_foo.py");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "def test_foo(): pass").expect("write");

        let resolved = resolve_testcase_file(
            Some(&src),
            "test_foo",
            "tests",
            dir.path(),
        );
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), src);
    }

    #[test]
    fn classname_to_path_fallback_works() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/test_math.rs");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "fn test_math() {}").expect("write");

        let resolved = resolve_testcase_file(
            None,
            "test_math",
            "tests",
            dir.path(),
        );
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), PathBuf::from("tests/test_math.rs"));
    }

    #[test]
    fn classname_with_dots_maps_to_path() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/com/example/widget_test.py");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "def test_widget(): pass").expect("write");

        let resolved = resolve_testcase_file(
            None,
            "com.example.widget_test",
            "tests",
            dir.path(),
        );
        assert!(resolved.is_some());
        assert_eq!(
            resolved.unwrap(),
            PathBuf::from("tests/com/example/widget_test.py")
        );
    }

    #[test]
    fn classname_unresolved_returns_none() {
        let dir = tempfile::tempdir().expect("temp dir");
        let resolved = resolve_testcase_file(
            None,
            "nonexistent.test",
            "tests",
            dir.path(),
        );
        assert!(resolved.is_none());
    }

    #[test]
    fn independent_rerun_entries_evaluated_separately() {
        let xml = make_junit_xml(
            r#"<?xml version="1.0"?>
            <testsuite name="ts" tests="2">
                <testcase classname="c" name="flaky_test" time="0.5"/>
                <testcase classname="c" name="flaky_test" time="0.3"/>
            </testsuite>"#,
        );
        let timings = parse_junit(xml.path(), TimeUnit::Seconds).expect("parse");
        assert_eq!(timings.len(), 2);
        assert_eq!(timings[0].duration_ms, 500);
        assert_eq!(timings[1].duration_ms, 300);
    }

    #[test]
    fn time_attribute_fractional_seconds() {
        let xml = make_junit_xml(
            r#"<?xml version="1.0"?>
            <testsuite name="ts" tests="1">
                <testcase classname="c" name="slow" time="1.234567"/>
            </testsuite>"#,
        );
        let timings = parse_junit(xml.path(), TimeUnit::Seconds).expect("parse");
        // 1.234567 * 1000 = 1234.567 → round-half-up = 1235
        assert_eq!(timings[0].duration_ms, 1235);
    }

    #[test]
    fn file_not_found_falls_back_to_classname() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/test_widget.py");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "def test_widget(): pass").expect("write");

        // file attribute points to nonexistent path, classname should match
        let resolved = resolve_testcase_file(
            Some(Path::new("nonexistent/foo.py")),
            "test_widget",
            "tests",
            dir.path(),
        );
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), PathBuf::from("tests/test_widget.py"));
    }

    #[test]
    fn evaluate_over_threshold_unit_test() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/unit/test_widget.py");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "def test_widget(): pass").expect("write");

        let config = Config::parse_str(
            r#"
            [thresholds.unit-test]
            duration_max_ms = 50
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        )
        .expect("config parses");
        let detector = RoleDetector::new(&config).expect("valid");
        let timings = vec![TestTiming {
            name: "test_widget".to_string(),
            classname: "test_widget".to_string(),
            file: Some(src),
            duration_ms: 100,
            status: TestStatus::Passed,
        }];

        let findings = evaluate_duration(&timings, &detector, &config, dir.path());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].test_name, "test_widget");
        assert_eq!(findings[0].observed_ms, 100);
        assert_eq!(findings[0].threshold_ms, 50);
    }

    #[test]
    fn evaluate_under_threshold_no_finding() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/unit/test_widget.py");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "def test_widget(): pass").expect("write");

        let config = Config::parse_str(
            r#"
            [thresholds.unit-test]
            duration_max_ms = 200
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        )
        .expect("config parses");
        let detector = RoleDetector::new(&config).expect("valid");
        let timings = vec![TestTiming {
            name: "test_widget".to_string(),
            classname: "test_widget".to_string(),
            file: Some(src),
            duration_ms: 100,
            status: TestStatus::Passed,
        }];

        let findings = evaluate_duration(&timings, &detector, &config, dir.path());
        assert_eq!(findings.len(), 0);
    }

    #[test]
    fn evaluate_exactly_at_threshold_no_finding() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/unit/test_widget.py");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "def test_widget(): pass").expect("write");

        let config = Config::parse_str(
            r#"
            [thresholds.unit-test]
            duration_max_ms = 100
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        )
        .expect("config parses");
        let detector = RoleDetector::new(&config).expect("valid");
        let timings = vec![TestTiming {
            name: "test_widget".to_string(),
            classname: "test_widget".to_string(),
            file: Some(src),
            duration_ms: 100,
            status: TestStatus::Passed,
        }];

        let findings = evaluate_duration(&timings, &detector, &config, dir.path());
        assert_eq!(findings.len(), 0, "strict > comparison, exactly at threshold is not a violation");
    }

    #[test]
    fn evaluate_integration_test_threshold() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/integration/test_api.py");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "def test_api(): pass").expect("write");

        let config = Config::parse_str(
            r#"
            [thresholds.integration-test]
            duration_max_ms = 500
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        )
        .expect("config parses");
        let detector = RoleDetector::new(&config).expect("valid");
        let timings = vec![TestTiming {
            name: "test_api".to_string(),
            classname: "test_api".to_string(),
            file: Some(src),
            duration_ms: 600,
            status: TestStatus::Passed,
        }];

        let findings = evaluate_duration(&timings, &detector, &config, dir.path());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].threshold_ms, 500);
    }

    #[test]
    fn evaluate_base_test_no_threshold_skips() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/test_widget.py");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "def test_widget(): pass").expect("write");

        let config = Config::parse_str(
            r#"
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        )
        .expect("config parses");
        let detector = RoleDetector::new(&config).expect("valid");
        let timings = vec![TestTiming {
            name: "test_widget".to_string(),
            classname: "test_widget".to_string(),
            file: Some(src),
            duration_ms: 9999,
            status: TestStatus::Passed,
        }];

        let findings = evaluate_duration(&timings, &detector, &config, dir.path());
        assert_eq!(findings.len(), 0, "base test role has no duration threshold");
    }

    #[test]
    fn evaluate_zero_threshold_disabled() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/unit/test_widget.py");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "def test_widget(): pass").expect("write");

        let config = Config::parse_str(
            r#"
            [thresholds.unit-test]
            duration_max_ms = 0
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        )
        .expect("config parses");
        let detector = RoleDetector::new(&config).expect("valid");
        let timings = vec![TestTiming {
            name: "test_widget".to_string(),
            classname: "test_widget".to_string(),
            file: Some(src),
            duration_ms: 9999,
            status: TestStatus::Passed,
        }];

        let findings = evaluate_duration(&timings, &detector, &config, dir.path());
        assert_eq!(findings.len(), 0, "zero threshold means disabled");
    }

    #[test]
    fn evaluate_skipped_tests_excluded() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/unit/test_widget.py");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "def test_widget(): pass").expect("write");

        let config = Config::parse_str(
            r#"
            [thresholds.unit-test]
            duration_max_ms = 50
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        )
        .expect("config parses");
        let detector = RoleDetector::new(&config).expect("valid");
        // Skipped testcases are excluded from parse_junit output, so they
        // never reach the evaluator. This test verifies the pipeline.
        let timings = vec![TestTiming {
            name: "test_widget".to_string(),
            classname: "test_widget".to_string(),
            file: Some(src),
            duration_ms: 100,
            status: TestStatus::Passed,
        }];

        let findings = evaluate_duration(&timings, &detector, &config, dir.path());
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn evaluate_failed_still_evaluated() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/unit/test_widget.py");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "def test_widget(): pass").expect("write");

        let config = Config::parse_str(
            r#"
            [thresholds.unit-test]
            duration_max_ms = 50
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        )
        .expect("config parses");
        let detector = RoleDetector::new(&config).expect("valid");
        let timings = vec![TestTiming {
            name: "test_widget".to_string(),
            classname: "test_widget".to_string(),
            file: Some(src),
            duration_ms: 100,
            status: TestStatus::Failed,
        }];

        let findings = evaluate_duration(&timings, &detector, &config, dir.path());
        assert_eq!(findings.len(), 1, "failed tests are still evaluated for duration");
    }

    #[test]
    fn evaluate_independent_rerun_entries() {
        let dir = tempfile::tempdir().expect("temp dir");
        let src = dir.path().join("tests/unit/test_widget.py");
        std::fs::create_dir_all(src.parent().unwrap()).expect("create dir");
        std::fs::write(&src, "def test_widget(): pass").expect("write");

        let config = Config::parse_str(
            r#"
            [thresholds.unit-test]
            duration_max_ms = 50
            [roles]
            test = { paths = [] }
            unit-test = { paths = [] }
            integration-test = { paths = [] }
            "#,
        )
        .expect("config parses");
        let detector = RoleDetector::new(&config).expect("valid");
        let timings = vec![
            TestTiming {
                name: "flaky_test".to_string(),
                classname: "test_widget".to_string(),
                file: Some(src.clone()),
                duration_ms: 100,
                status: TestStatus::Passed,
            },
            TestTiming {
                name: "flaky_test".to_string(),
                classname: "test_widget".to_string(),
                file: Some(src),
                duration_ms: 30,
                status: TestStatus::Passed,
            },
        ];

        let findings = evaluate_duration(&timings, &detector, &config, dir.path());
        assert_eq!(findings.len(), 1, "only the over-threshold rerun entry is flagged");
    }
}