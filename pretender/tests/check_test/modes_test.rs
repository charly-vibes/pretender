//! `check` exit-policy and mode tests (gate/tiered/guidance).
//!
//! # Responsibilities
//! Pin per-mode exit codes, the advisory-lease behavior (GH #35), the
//! fail-closed missing-lease diagnostics, config-supplied mode precedence,
//! and clean-function visibility under gate mode.
//!
//! # Rationale
//! Moved verbatim from `tests/cli_test.rs` (wave 3 per-command split);
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms.
//! The check subprocess invocation is concentrated in `run_check`; exit-code
//! and stdout/stderr assertions stay in the tests.

use crate::common::*;
use std::path::Path;
use std::process::Output;

/// Runs `pretender check <staged> [args...]` with color suppressed and
/// returns the raw output; exit-code policy stays with the caller.
fn run_check(staged: &Path, args: &[&str]) -> Output {
    pretender_cmd()
        .arg("check")
        .arg(staged)
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"))
}

#[test]
fn test_check_default_gate_mode_fails_on_violation() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = run_check(&staged, &[]);

    // Gate is now the fail-closed default (pretender-1te): violations EXIT 1
    assert_eq!(
        output.status.code(),
        Some(1),
        "default gate mode must fail on violations; stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_check_tiered_mode_shows_advisory_not_violation() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = run_check(
        &staged,
        &["--mode", "tiered", "--advisory-until", "2099-12-31"],
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ADVISORY"),
        "tiered mode should show ADVISORY; stdout: {stdout}"
    );
    assert!(
        !stdout.contains("VIOLATION"),
        "tiered mode should not show VIOLATION; stdout: {stdout}"
    );
    assert!(
        !stdout.contains("✗"),
        "tiered mode should not show ✗; stdout: {stdout}"
    );
}

#[test]
fn test_check_tiered_missing_lease_fails_closed_with_diagnostic() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = run_check(&staged, &["--mode", "tiered"]);

    assert_eq!(
        output.status.code(),
        Some(1),
        "missing lease must fail closed; stdout: {} stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("tiered") && stderr.contains("advisory_until"),
        "stderr must name the mode and the config remediation; got: {stderr}"
    );
    assert!(
        stderr.contains("--advisory-until"),
        "stderr must mention the CLI lease flag; got: {stderr}"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("all green"),
        "stdout should still carry the normal report; got: {stdout}"
    );
}

#[test]
fn test_check_tiered_missing_lease_json_ok_false_with_warning() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = run_check(&staged, &["--mode", "tiered", "--format", "json"]);

    assert_eq!(output.status.code(), Some(1));
    let json: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))
        .unwrap_or_else(|e| panic!("stdout must remain valid JSON: {e:?}"));
    assert_eq!(
        json["ok"],
        serde_json::json!(false),
        "envelope ok must agree with the exit code; got: {json}"
    );
    let warnings = json["warnings"]
        .as_array()
        .unwrap_or_else(|| panic!("warnings array"));
    assert!(
        warnings.iter().any(|w| {
            let msg = w["message"].as_str().unwrap_or("");
            msg.contains("tiered") && msg.contains("advisory_until")
        }),
        "envelope must carry a structured lease warning; got: {warnings:?}"
    );
}

#[test]
fn test_check_gate_mode_no_lease_no_advisory_diagnostic() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = run_check(&staged, &["--mode", "gate"]);

    assert!(output.status.success(), "gate + green files must pass");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("advisory"),
        "gate mode must not emit lease diagnostics; got: {stderr}"
    );
}

#[test]
fn test_check_gate_mode_shows_violation() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = run_check(&staged, &["--mode", "gate"]);

    // Gate violations must fail (exit 1)...
    assert_eq!(
        output.status.code(),
        Some(1),
        "gate mode must exit 1 on violations; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // ...and show the violation — assert only the label, color codes vary by terminal
    assert!(
        stdout.contains("VIOLATION"),
        "gate mode should show VIOLATION; stdout: {stdout}"
    );
}

#[test]
fn test_check_guidance_mode_exits_zero_on_violation() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = run_check(
        &staged,
        &["--mode", "guidance", "--advisory-until", "2099-12-31"],
    );

    assert_eq!(
        output.status.code(),
        Some(0),
        "guidance mode must exit 0 even when violations exist; stdout: {}",
        String::from_utf8_lossy(&output.stdout),
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // guidance mode uses ⚠ ADVISORY, not ✗ VIOLATION
    assert!(
        stdout.contains("ADVISORY"),
        "guidance mode should surface violations as advisories; got: {stdout}",
    );
    assert!(
        !stdout.contains("VIOLATION"),
        "guidance mode must not label findings as violations; got: {stdout}",
    );
}

#[test]
fn test_check_gate_mode_from_config_fails_on_violation() {
    let dir = tempdir();
    let staged = dir.join("python_violator.py");
    std::fs::copy(source_fixture("python_violator.py"), &staged)
        .unwrap_or_else(|e| panic!("copy fixture: {e:?}"));
    std::fs::write(dir.join("pretender.toml"), "[pretender]\nmode = \"gate\"\n")
        .unwrap_or_else(|e| panic!("write config: {e:?}"));

    // Config-supplied mode must win — build raw so the helper's lease/mode
    // flags don't shadow it (pretender-1te)
    let output = pretender_cmd()
        .arg("check")
        .arg(&staged)
        .env("NO_COLOR", "1")
        .current_dir(&dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    // The config-driven gate must both fail...
    assert_eq!(output.status.code(), Some(1));
    // ...and report the violation like the --mode gate spelling does.
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("VIOLATION"),
        "config-selected gate mode should show VIOLATION; stdout: {stdout}"
    );
}

#[test]
fn test_check_tiered_human_output_highlights_yellow_band() {
    let dir = tempdir();
    let staged = dir.join("yellow.py");
    std::fs::write(
        dir.join("pretender.toml"),
        "[thresholds]\ncyclomatic_max = 99\ncognitive_max = 99\n",
    )
    .unwrap_or_else(|e| panic!("write config: {e:?}"));
    std::fs::write(
        &staged,
        "def yellow_band(value):\n    if value > 0:\n        pass\n    if value > 1:\n        pass\n    if value > 2:\n        pass\n    if value > 3:\n        pass\n    if value > 4:\n        pass\n    if value > 5:\n        pass\n    if value > 6:\n        pass\n    if value > 7:\n        pass\n    if value > 8:\n        pass\n    if value > 9:\n        pass\n    if value > 10:\n        pass\n    return value\n",
    )
    .unwrap_or_else(|e| panic!("write source: {e:?}"));

    let output = check_in(&dir, &staged)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("⚠"), "stdout: {stdout}");
    assert!(
        stdout.contains("yellow_band(): cyclomatic 12 (yellow)"),
        "stdout: {stdout}"
    );
}

#[test]
fn test_check_gate_mode_fails_on_violation() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = run_check(&staged, &["--mode", "gate"]);

    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn test_check_default_hides_clean_functions() {
    // python_simple has passing functions — default output should not list them
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = run_check(&staged, &["--mode", "gate"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    // No per-function metric lines should appear for a clean file
    assert!(
        !stdout
            .lines()
            .any(|l| l.starts_with("  ") && l.contains("cyclomatic=")),
        "clean functions should be hidden by default; stdout: {stdout}"
    );
}

#[test]
fn test_check_verbose_shows_all_functions() {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = run_check(&staged, &["--mode", "gate", "--show-all"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.lines().any(|l| l.contains("cyclomatic=")),
        "--show-all should show per-function metrics; stdout: {stdout}"
    );
}
