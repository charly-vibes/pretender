//! Integration tests for the `report` subcommand's output formats.
//!
//! # Responsibilities
//! Pin that `report` renders the cached last check (JSON or human) as
//! markdown to stdout, writes `--format html` reports to the `--output`
//! file without stdout noise, and fails with exit 1 and a clear stderr
//! when no cached report exists.
//!
//! # Rationale
//! Moved verbatim from `tests/cli_test.rs` (wave 3 per-command split);
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms.

mod common;

use common::*;

#[test]
fn test_report_markdown_reads_last_json_check() {
    let dir = tempdir();
    let staged = dir.join("python_violator.py");
    std::fs::copy(source_fixture("python_violator.py"), &staged)
        .unwrap_or_else(|e| panic!("copy fixture: {e:?}"));

    let check_output = check_in(&dir, &staged)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap_or_else(|e| panic!("run json check: {e:?}"));
    assert!(
        check_output.status.success(),
        "json check failed: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );

    let output = report_in(&dir)
        .arg("--format")
        .arg("markdown")
        .output()
        .unwrap_or_else(|e| panic!("run report: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Pretender report"), "stdout: {stdout}");
    assert!(stdout.contains("python_violator.py"), "stdout: {stdout}");
    assert!(stdout.contains("too_many_params"), "stdout: {stdout}");
    assert!(stdout.contains("cyclomatic"), "stdout: {stdout}");
}

#[test]
fn test_report_html_writes_output_file() {
    let dir = tempdir();
    let staged = dir.join("python_violator.py");
    let out_path = dir.join("pretender-report.html");
    std::fs::copy(source_fixture("python_violator.py"), &staged)
        .unwrap_or_else(|e| panic!("copy fixture: {e:?}"));

    let check_output = check_in(&dir, &staged)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap_or_else(|e| panic!("run json check: {e:?}"));
    assert!(
        check_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );

    let output = report_in(&dir)
        .arg("--format")
        .arg("html")
        .arg("--output")
        .arg(&out_path)
        .output()
        .unwrap_or_else(|e| panic!("run report: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let html = std::fs::read_to_string(&out_path)
        .unwrap_or_else(|e| panic!("html report exists: {e:?}"));
    assert!(html.contains("<!doctype html>"), "html: {html}");
    assert!(html.contains("python_violator.py"), "html: {html}");
    assert!(html.contains("too_many_params"), "html: {html}");
}

#[test]
fn test_report_markdown_reads_last_human_check() {
    let dir = tempdir();
    let staged = dir.join("python_violator.py");
    std::fs::copy(source_fixture("python_violator.py"), &staged)
        .unwrap_or_else(|e| panic!("copy fixture: {e:?}"));

    let check_output = check_in(&dir, &staged)
        .output()
        .unwrap_or_else(|e| panic!("run human check: {e:?}"));
    assert!(
        check_output.status.success(),
        "human check failed: {}",
        String::from_utf8_lossy(&check_output.stderr)
    );

    let output = report_in(&dir)
        .arg("--format")
        .arg("markdown")
        .output()
        .unwrap_or_else(|e| panic!("run report: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("# Pretender report"), "stdout: {stdout}");
    assert!(stdout.contains("python_violator.py"), "stdout: {stdout}");
}

#[test]
fn test_report_fails_without_cached_report() {
    let dir = tempdir();

    let output = report_in(&dir)
        .output()
        .unwrap_or_else(|e| panic!("run report: {e:?}"));

    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("failed to read last check report"),
        "stderr: {stderr}"
    );
}
