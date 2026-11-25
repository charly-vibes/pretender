//! Mutation-preview and smell-weighting tests.
//!
//! # Responsibilities
//! Pin `mutation --dry-run` exit policy for files and directories and the
//! smell call-weight effect on ABC metrics surfaced via `check --format json`.
//!
//! # Rationale
//! Bodies moved verbatim from `tests/cli_test.rs` (wave-3 split) with
//! expect/unwrap calls converted to sanctioned panic-on-failure forms.

use crate::common::*;

#[test]
fn test_mutation_dry_run_python() {
    let output = pretender_cmd()
        .args([
            "mutation",
            "--dry-run",
            source_fixture("python_simple.py")
                .to_str()
                .unwrap_or_else(|| panic!("source fixture path is not valid UTF-8")),
        ])
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "dry-run should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("mutmut"),
        "dry-run should name the tool; got: {stdout}"
    );
    assert!(
        stdout.contains("Planned mutants") || stdout.contains("No mutation sites"),
        "dry-run should list planned mutants; got: {stdout}"
    );
}

#[test]
fn test_mutation_dry_run_directory() {
    // Regression: `mutation --dry-run <dir>` failed with
    // "no supported source files found" because directories were passed
    // to detect_language() which checks file extensions.
    let dir = source_fixture("");
    let output = pretender_cmd()
        .args([
            "mutation",
            "--dry-run",
            dir.to_str()
                .unwrap_or_else(|| panic!("source fixture path is not valid UTF-8")),
        ])
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "mutation --dry-run should succeed on a directory; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_smell_call_weights_elevate_abc() {
    let (_dir, path) = stage_fixture("python_smell_calls.py");
    let output = pretender_cmd()
        .arg("check")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("run pretender: {e:?}"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid json: {e}\nstdout: {stdout}"));
    let units = json["data"]["files"][0]["units"]
        .as_array()
        .unwrap_or_else(|| panic!("units array"));
    let abc_for = |name: &str| -> f64 {
        units
            .iter()
            .find(|u| u["name"] == name)
            .unwrap_or_else(|| panic!("unit {name} not found"))["metrics"]["abc"]
            .as_f64()
            .unwrap_or_else(|| panic!("abc metric for {name}"))
    };
    // smell_eval: A=1, B=0, C=5 (eval weight=5.0) → sqrt(26) ≈ 5.099
    let eval_abc = abc_for("smell_eval");
    assert!(
        (eval_abc - 5.099).abs() < 0.01,
        "smell_eval abc={eval_abc}, expected ≈5.099 (sqrt(26))"
    );
    // smell_exec: A=0, B=0, C=5 (exec weight=5.0) → 5.0
    let exec_abc = abc_for("smell_exec");
    assert!(
        (exec_abc - 5.0).abs() < 0.01,
        "smell_exec abc={exec_abc}, expected 5.0"
    );
    // smell_compile: A=1, B=0, C=3 (compile weight=3.0) → sqrt(10) ≈ 3.162
    let compile_abc = abc_for("smell_compile");
    assert!(
        (compile_abc - 3.162).abs() < 0.01,
        "smell_compile abc={compile_abc}, expected ≈3.162 (sqrt(10))"
    );
}
