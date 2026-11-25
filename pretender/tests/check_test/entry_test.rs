//! `check` entry-point and format acceptance tests.
//!
//! # Responsibilities
//! Pin the default working-directory behavior, human/JSON output basics,
//! the JSON envelope shape, clean-run exit codes, format rejection, the
//! `--staged` short-circuit paths, and binary-file skipping.
//!
//! # Rationale
//! Moved verbatim from `tests/cli_test.rs` (wave 3 per-command split);
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms.

use crate::common::*;
use std::path::PathBuf;

#[test]
fn test_check_defaults_to_current_dir() {
    let dir = tempdir();
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join("python_simple.py");
    let dest = dir.join("python_simple.py");
    std::fs::copy(&src, &dest).unwrap_or_else(|e| panic!("copy fixture: {e:?}"));

    let output = check_default(&dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("python_simple.py"), "stdout: {stdout}");
}

#[test]
fn test_check_command_human_output() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged).output().unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("python_simple.py"), "stdout: {stdout}");
    // Default mode hides clean functions; --verbose would show them
    assert!(
        !stdout.contains("cyclomatic="),
        "default should not show per-function metrics for clean files; stdout: {stdout}"
    );
}

#[test]
fn test_check_command_json_output() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|e| panic!("stdout should be valid json: {e:?}"));

    assert_eq!(json["data"]["files"].as_array().map(Vec::len), Some(1));
    assert_eq!(
        json["data"]["files"][0]["path"].as_str(),
        Some(staged.to_string_lossy().as_ref())
    );
    assert_eq!(
        json["data"]["files"][0]["units"][0]["name"].as_str(),
        Some("simple")
    );
    assert_eq!(
        json["data"]["files"][0]["units"][2]["metrics"]["cyclomatic"].as_u64(),
        Some(6)
    );
}

#[test]
fn test_check_json_output_has_envelope_shape() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|e| panic!("stdout should be valid json: {e:?}"));

    // Verify genesis envelope shape
    assert!(json.get("ok").is_some(), "envelope must have 'ok' field");
    assert_eq!(json["ok"], true, "successful check should have ok=true");
    assert_eq!(
        json["envelope_version"], "0.1",
        "envelope_version should be 0.1"
    );
    assert_eq!(
        json["envelope_kind"], "check",
        "envelope_kind should be 'check'"
    );
    assert!(
        json.get("cli_version").is_some(),
        "envelope must have 'cli_version'"
    );
    assert!(
        json.get("data").is_some(),
        "envelope must have 'data' field"
    );
    assert!(
        json.get("warnings").is_some(),
        "envelope must have 'warnings' field"
    );
    assert!(
        json.get("hints").is_some(),
        "envelope must have 'hints' field"
    );
    assert!(
        json.get("meta").is_some(),
        "envelope must have 'meta' field"
    );
    // Verify existing data is preserved within the envelope
    assert_eq!(
        json["data"]["files"].as_array().map(Vec::len),
        Some(1),
        "data.files should have 1 file"
    );
}

#[test]
fn test_check_exits_zero_when_clean() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged).output().unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(
        output.status.code(),
        Some(0),
        "expected exit 0 on clean fixture; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_check_format_junit_is_rejected() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged)
        .arg("--format")
        .arg("junit")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        !output.status.success(),
        "junit format should be rejected (removed until implemented)"
    );
}

#[test]
fn test_check_staged_no_files_prints_message() {
    let dir = tempdir();
    git_init(&dir);
    // No staged files — just an empty git repo with a source file not added
    std::fs::write(dir.join("hello.py"), "def hello():\n    pass\n").unwrap_or_else(|e| panic!("write file: {e:?}"));

    let output = check_in(&dir, &dir)
        .arg("--staged")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(
        output.status.code(),
        Some(0),
        "should exit 0 with no staged files; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("No staged files to check"),
        "should print message when no staged files; stdout: {stdout}"
    );
}

#[test]
fn test_check_staged_no_files_json_envelope() {
    // pretender-xgt: the short-circuit path must emit a parseable JSON
    // envelope when --format json is requested, same shape as a normal run.
    let dir = tempdir();
    git_init(&dir);
    std::fs::write(dir.join("hello.py"), "def hello():\n    pass\n").unwrap_or_else(|e| panic!("write file: {e:?}"));

    let output = check_in(&dir, &dir)
        .arg("--staged")
        .arg("--format")
        .arg("json")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(
        output.status.code(),
        Some(0),
        "should exit 0 with no staged files; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("--format json must emit a JSON envelope even when nothing is staged: {e:?}"));

    assert_eq!(json["ok"], true, "no findings on empty diff, ok=true");
    assert_eq!(
        json["envelope_kind"], "check",
        "envelope_kind should be 'check'"
    );
    assert_eq!(
        json["data"]["files"].as_array().map(Vec::len),
        Some(0),
        "data.files should be an empty array; got: {json}"
    );
}

#[test]
fn test_check_skips_binary_files() {
    let dir = tempdir();
    // Write a binary file (null bytes are not valid UTF-8)
    let binary = dir.join("artifact.bin");
    std::fs::write(&binary, b"\x00\x01\x02\x03\xff\xfe").unwrap_or_else(|e| panic!("write binary file: {e:?}"));

    let output = check(&binary).output().unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(
        output.status.code(),
        Some(0),
        "check should exit 0 when given a binary file; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_check_skips_binary_files_in_directory() {
    let dir = tempdir();
    // Write a source file alongside a binary file
    let source = dir.join("hello.py");
    std::fs::write(&source, "def hello():\n    pass\n").unwrap_or_else(|e| panic!("write source: {e:?}"));
    let binary = dir.join("artifact.bin");
    std::fs::write(&binary, b"\x00\x01\x02\x03\xff\xfe").unwrap_or_else(|e| panic!("write binary file: {e:?}"));

    let output = check(&dir).output().unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(
        output.status.code(),
        Some(0),
        "check should exit 0 when directory contains binary files; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
