//! Integration tests for `test_report::evaluate_duration` (moved from the
//! former in-file `#[cfg(test)]` module, pretender-5zo.15.1). The
//! near-identical original fixtures share one `eval_with` core; the
//! uniform no-finding / finding families are table-driven so no
//! structurally identical test cluster remains.

use pretender::config::Config;
use pretender::role_resolution::RoleDetector;
use pretender::test_report::{evaluate_duration, TestDurationFinding, TestStatus, TestTiming};
use std::path::{Path, PathBuf};

/// The original tests' `[thresholds]` fragment for a 50 ms unit-test cap.
const UNIT_T50: &str = "[thresholds.unit-test]\nduration_max_ms = 50";

/// The original tests' fixture path (all resolution needs is existence).
const UNIT_PY: &str = "tests/unit/test_widget.py";

/// A [`TestTiming`] over `src` with `name` (classname mirrors the name,
/// as in the original fixtures).
fn timing(name: &str, src: &Path) -> TestTiming {
    TestTiming {
        name: name.to_string(),
        classname: name.to_string(),
        file: Some(src.to_path_buf()),
        duration_ms: 0,
        status: TestStatus::Passed,
    }
}

/// Create `rel` under `dir` (existence is all resolution needs).
fn write_fixture(dir: &Path, rel: &str) -> PathBuf {
    let src = dir.join(rel);
    std::fs::create_dir_all(src.parent().unwrap_or_else(|| panic!("no parent: {rel}")))
        .unwrap_or_else(|e| panic!("mkdir {rel}: {e:?}"));
    std::fs::write(&src, "def test_x(): pass").unwrap_or_else(|e| panic!("write {rel}: {e:?}"));
    src
}

/// Per-case spec for [`eval_with`]: the `[thresholds]` fragment and the
/// fixture path to create under the temp root.
struct Case {
    thresholds: &'static str,
    src_rel: &'static str,
}

/// Evaluate timings over a freshly written fixture file, under a config
/// whose roles are all empty-path (the original tests' config shape).
fn eval_with(spec: Case, build: impl FnOnce(&Path) -> Vec<TestTiming>) -> Vec<TestDurationFinding> {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("temp dir: {e:?}"));
    let src = write_fixture(dir.path(), spec.src_rel);
    let toml = format!(
        "{}\n[roles]\ntest = {{ paths = [] }}\nunit-test = {{ paths = [] }}\nintegration-test = {{ paths = [] }}\n",
        spec.thresholds
    );
    let config = Config::parse_str(&toml).unwrap_or_else(|e| panic!("config parses: {e:?}"));
    let detector = RoleDetector::new(&config).unwrap_or_else(|e| panic!("valid: {e:?}"));
    evaluate_duration(&build(&src), &detector, &config, dir.path())
}

#[test]
fn evaluate_over_threshold_unit_test() {
    let findings = eval_with(
        Case {
            thresholds: UNIT_T50,
            src_rel: UNIT_PY,
        },
        |src| {
            vec![TestTiming {
                duration_ms: 100,
                ..timing("test_widget", src)
            }]
        },
    );
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].test_name, "test_widget");
    assert_eq!(findings[0].observed_ms, 100);
    assert_eq!(findings[0].threshold_ms, 50);
}

#[test]
fn evaluate_under_or_disabled_threshold_no_finding() {
    // Under, exactly-at (strict >), base-test role, and disabled (zero)
    // thresholds all yield no finding — the original four cases.
    let cases = [
        (
            "under threshold",
            "[thresholds.unit-test]\nduration_max_ms = 200",
            100,
            TestStatus::Passed,
        ),
        (
            "exactly at threshold",
            "[thresholds.unit-test]\nduration_max_ms = 100",
            100,
            TestStatus::Passed,
        ),
        ("base test role", "", 9999, TestStatus::Passed),
        (
            "zero threshold disabled",
            "[thresholds.unit-test]\nduration_max_ms = 0",
            9999,
            TestStatus::Passed,
        ),
    ];
    for (label, thresholds, duration_ms, status) in cases {
        let findings = eval_with(
            Case {
                thresholds,
                src_rel: UNIT_PY,
            },
            |src| {
                vec![TestTiming {
                    duration_ms,
                    status,
                    ..timing("test_widget", src)
                }]
            },
        );
        assert!(findings.is_empty(), "{label}");
    }
}

#[test]
fn evaluate_failed_and_pipeline_entries_counted() {
    // Failed tests are still evaluated for duration; skipped testcases
    // never reach the evaluator (parse_junit excludes them), so a Passed
    // entry is the pipeline's proof.
    let cases = [
        ("failed still evaluated", TestStatus::Failed),
        ("passed pipeline entry", TestStatus::Passed),
    ];
    for (label, status) in cases {
        let findings = eval_with(
            Case {
                thresholds: UNIT_T50,
                src_rel: UNIT_PY,
            },
            |src| {
                vec![TestTiming {
                    duration_ms: 100,
                    status,
                    ..timing("test_widget", src)
                }]
            },
        );
        assert_eq!(findings.len(), 1, "{label}");
    }
}

#[test]
fn evaluate_integration_test_threshold() {
    let findings = eval_with(
        Case {
            thresholds: "[thresholds.integration-test]\nduration_max_ms = 500",
            src_rel: "tests/integration/test_api.py",
        },
        |src| {
            vec![TestTiming {
                duration_ms: 600,
                ..timing("test_api", src)
            }]
        },
    );
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].threshold_ms, 500);
}

#[test]
fn evaluate_independent_rerun_entries() {
    let findings = eval_with(
        Case {
            thresholds: UNIT_T50,
            src_rel: UNIT_PY,
        },
        |src| {
            vec![
                TestTiming {
                    duration_ms: 100,
                    ..timing("flaky_test", src)
                },
                TestTiming {
                    duration_ms: 30,
                    ..timing("flaky_test", src)
                },
            ]
        },
    );
    assert_eq!(
        findings.len(),
        1,
        "only the over-threshold rerun entry is flagged"
    );
}
