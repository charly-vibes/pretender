//! `check` human-output and SARIF rendering tests.
//!
//! # Responsibilities
//! Pin `--output` file writing (JSON and uncolored text), violation surface
//! strings, cognitive-violation reporting, numeric rounding in human output,
//! ABC decimal formatting, and the SARIF 2.1.0 structure.
//!
//! # Rationale
//! Moved verbatim from `tests/cli_test.rs` (wave 3 per-command split);
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms.

use crate::common::*;

#[test]
fn test_check_output_flag_writes_to_file() {
    let (_dir, staged) = stage_fixture("python_simple.py");
    let out_path = staged.with_file_name("report.json");

    let output = check(&staged)
        .arg("--format")
        .arg("json")
        .arg("--output")
        .arg(&out_path)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty when --output is set; got: {}",
        String::from_utf8_lossy(&output.stdout)
    );

    let written = std::fs::read_to_string(&out_path)
        .unwrap_or_else(|e| panic!("report file should exist: {e:?}"));
    let json: serde_json::Value = serde_json::from_str(&written)
        .unwrap_or_else(|e| panic!("report file should be valid json: {e:?}"));
    assert_eq!(json["data"]["files"].as_array().map(Vec::len), Some(1));
}

#[test]
fn test_check_human_output_to_file_is_uncolored() {
    let (_dir, staged) = stage_fixture("python_simple.py");
    let out_path = staged.with_file_name("report.txt");

    let output = check(&staged)
        .arg("--output")
        .arg(&out_path)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let written = std::fs::read_to_string(&out_path)
        .unwrap_or_else(|e| panic!("report file should exist: {e:?}"));
    assert!(
        !written.contains('\u{1b}'),
        "report written to file must not contain ANSI escape sequences: {written:?}"
    );
}

#[test]
fn test_check_human_output_surfaces_violations() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = check(&staged)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Default mode is tiered → ⚠ ADVISORY (not ✗ VIOLATION)
    assert!(stdout.contains("⚠"), "stdout: {stdout}");
    assert!(stdout.contains("python_violator.py"), "stdout: {stdout}");
    assert!(
        stdout.contains("ADVISORY"),
        "tiered mode should mark violations as advisory; got: {stdout}"
    );
    assert!(
        stdout.contains("params"),
        "human output should name the violated metric; got: {stdout}"
    );
    assert!(
        !stdout.contains('\u{1b}'),
        "NO_COLOR should suppress ANSI codes; got: {stdout:?}"
    );
}

#[test]
fn test_check_human_output_reports_cognitive_violations() {
    let (_dir, staged) = stage_fixture("python_cognitive.py");

    let output = check(&staged)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("deeply_nested"), "stdout: {stdout}");
    assert!(stdout.contains("cognitive"), "stdout: {stdout}");
}

#[test]
fn test_check_human_output_rounds_violation_values() {
    // python_violator triggers integer violations; ensure they display without decimals
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = pretender_cmd()
        .arg("check")
        .arg(&staged)
        .arg("--mode")
        .arg("gate")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Integer violations should display without trailing .00 (default app params_max = 4)
    assert!(
        stdout.contains("params 6 > 4"),
        "integer violation should display without decimals; stdout: {stdout}"
    );
}

#[test]
fn test_check_human_output_abc_rounds_to_two_decimal_places() {
    // Write a fixture that reliably triggers a fractional ABC violation
    // A function with assignments + branches + calls produces a non-integer ABC score
    let (dir, path) = write_temp_file(
        "abc_fixture.py",
        r#"
def complex_function(x, y, z):
    a = x + 1
    b = y * 2
    c = z - 1
    if a > 0:
        result = some_call(a, b)
        result2 = other_call(b, c)
        result3 = third_call(a, c)
        return result + result2 + result3
    elif b > 0:
        return another_call(a, b, c)
    else:
        return fallback(x, y, z)
"#,
    );
    // Write a config that sets a very low abc threshold to force a violation
    std::fs::write(dir.join("pretender.toml"), "[thresholds]\nabc_max = 1\n")
        .unwrap_or_else(|e| panic!("write config: {e:?}"));

    let output = pretender_cmd()
        .arg("check")
        .arg(&path)
        .arg("--mode")
        .arg("gate")
        .env("NO_COLOR", "1")
        .current_dir(&dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Violation line must exist and must not contain a long float
    let abc_violation = stdout
        .lines()
        .find(|l| l.contains("VIOLATION") && l.contains("abc"));
    assert!(
        abc_violation.is_some(),
        "expected abc violation; stdout: {stdout}"
    );
    let line = abc_violation.unwrap_or_else(|| panic!("abc violation line missing"));
    let has_long_float = line.split_whitespace().any(|tok| {
        if let Some(idx) = tok.find('.') {
            tok[idx + 1..].len() > 2
        } else {
            false
        }
    });
    assert!(
        !has_long_float,
        "abc violation line has excessive decimal places: {line}"
    );
}

#[test]
fn test_check_sarif_output_structure() {
    let (_dir, staged) = stage_fixture("python_violator.py");

    let output = check(&staged)
        .args(["--format", "sarif"])
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success() || output.status.code() == Some(1),
        "sarif format should not exit with error 2; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let sarif: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("stdout should be valid SARIF JSON: {e:?}"));

    assert_eq!(
        sarif["version"].as_str(),
        Some("2.1.0"),
        "version must be 2.1.0"
    );
    assert!(sarif["$schema"].as_str().is_some(), "must have $schema");

    let runs = sarif["runs"]
        .as_array()
        .unwrap_or_else(|| panic!("must have runs array"));
    assert_eq!(runs.len(), 1, "must have exactly one run");

    let run = &runs[0];
    assert_eq!(
        run["tool"]["driver"]["name"].as_str(),
        Some("pretender"),
        "tool.driver.name must be 'pretender'"
    );

    let rules = run["tool"]["driver"]["rules"]
        .as_array()
        .unwrap_or_else(|| panic!("must have rules array"));
    assert!(
        !rules.is_empty(),
        "rules must not be empty when violations exist"
    );
    for rule in rules {
        assert!(rule["id"].as_str().is_some(), "each rule must have an id");
    }

    let results = run["results"]
        .as_array()
        .unwrap_or_else(|| panic!("must have results array"));
    assert!(
        !results.is_empty(),
        "results must not be empty for a file with violations"
    );
    for result in results {
        assert_result_shape(result);
    }
}

/// Every result carries ruleId, message.text, and one physical location with
/// uri + startLine — the minimum a SARIF consumer needs to route a finding.
fn assert_result_shape(result: &serde_json::Value) {
    assert!(
        result["ruleId"].as_str().is_some(),
        "each result must have ruleId"
    );
    assert!(
        result["message"]["text"].as_str().is_some(),
        "each result must have message.text"
    );
    let locations = result["locations"]
        .as_array()
        .unwrap_or_else(|| panic!("each result must have locations"));
    assert!(
        !locations.is_empty(),
        "each result must have at least one location"
    );
    let phys = &locations[0]["physicalLocation"];
    assert!(
        phys["artifactLocation"]["uri"].as_str().is_some(),
        "must have uri"
    );
    assert!(
        phys["region"]["startLine"].as_i64().is_some(),
        "must have startLine"
    );
}
