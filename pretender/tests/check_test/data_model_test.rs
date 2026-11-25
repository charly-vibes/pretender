//! `check` data-model rule wiring tests.
//!
//! # Responsibilities
//! Stage data-model fixtures and pin that check surfaces data-model
//! violations in JSON with correct locations, that the test-role threshold
//! disables excessive-fields, and that SARIF results carry fixture lines.
//!
//! # Rationale
//! Moved verbatim from `tests/cli_test.rs` (wave 3 per-command split);
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms.
//! The data-model staging helpers live here because only these tests use
//! them (harness purity: shared helpers stay in `tests/common/mod.rs`).

use crate::common::*;
use std::path::{Path, PathBuf};

// ---- data model helpers (used only by these tests) ---------------------------------

fn data_model_source_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn stage_data_model_fixture(dir: &Path, fixture: &str) -> PathBuf {
    let staged = dir.join(fixture);
    std::fs::copy(data_model_source_fixture(fixture), &staged).unwrap_or_else(|e| panic!("copy fixture: {e:?}"));
    staged
}

fn data_model_fixture_dir(fixture: &str) -> (PathBuf, PathBuf) {
    let dir = tempdir();
    let staged = stage_data_model_fixture(&dir, fixture);
    std::fs::copy(
        data_model_source_fixture("data_model_pretender.toml"),
        dir.join("pretender.toml"),
    )
    .unwrap_or_else(|e| panic!("copy fixture toml: {e:?}"));
    (dir, staged)
}


#[test]
fn test_check_data_model_violations_json() {
    // §7.1: a 10-field struct with an `email: String` field emits both
    // `excessive_fields` (actual=10, limit=10, line = type def start) and
    // `primitive_field` (actual=1, limit=1, line = field start) in JSON.
    let (dir, staged) = data_model_fixture_dir("rust_data_model_sample.rs");

    let output = check_in(&dir, &staged)
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
    let violations = json["data"]["files"][0]["file_violations"]
        .as_array()
        .unwrap_or_else(|| panic!("file_violations array"));

    let excessive = violations
        .iter()
        .find(|v| v["metric"] == "excessive_fields")
        .unwrap_or_else(|| panic!("excessive_fields violation expected in json output"));
    assert_eq!(excessive["actual"].as_f64(), Some(10.0));
    assert_eq!(excessive["limit"].as_f64(), Some(10.0));
    assert_eq!(
        excessive["line"].as_u64(),
        Some(6),
        "line must be the type definition start line"
    );

    let primitive = violations
        .iter()
        .find(|v| v["metric"] == "primitive_field")
        .unwrap_or_else(|| panic!("primitive_field violation expected in json output"));
    assert_eq!(primitive["actual"].as_f64(), Some(1.0));
    assert_eq!(primitive["limit"].as_f64(), Some(1.0));
    assert_eq!(
        primitive["line"].as_u64(),
        Some(8),
        "line must be the field start line"
    );
}

#[test]
fn test_check_test_role_threshold_disables_excessive_fields() {
    // §7.2: `[thresholds.test] excessive_fields_max = 0` silences the check
    // for the pragma-assigned test-role fixture, while the default test
    // threshold (10) still flags the same 10-field struct (role merge proof).
    let dir = tempdir();
    let staged = stage_data_model_fixture(&dir, "rust_test_role_data_model_sample.rs");

    std::fs::write(dir.join("pretender.toml"), "[pretender]\n").unwrap_or_else(|e| panic!("write default config: {e:?}"));
    let baseline = check_in(&dir, &staged)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));
    let baseline_json: serde_json::Value =
        serde_json::from_slice(&baseline.stdout).unwrap_or_else(|e| panic!("baseline stdout should be valid json: {e:?}"));
    let baseline_violations = baseline_json["data"]["files"][0]["file_violations"]
        .as_array()
        .unwrap_or_else(|| panic!("file_violations array"));
    assert!(
        baseline_violations
            .iter()
            .any(|v| v["metric"] == "excessive_fields"),
        "default test threshold must flag the 10-field struct; violations: {baseline_violations:?}"
    );

    std::fs::write(
        dir.join("pretender.toml"),
        "[thresholds.test]\nexcessive_fields_max = 0\n",
    )
    .unwrap_or_else(|e| panic!("write override config: {e:?}"));
    let output = check_in(&dir, &staged)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|e| panic!("stdout should be valid json: {e:?}"));
    let violations = json["data"]["files"][0]["file_violations"]
        .as_array()
        .unwrap_or_else(|| panic!("file_violations array"));
    assert!(
        violations.iter().all(|v| v["metric"] != "excessive_fields"),
        "test-role excessive_fields_max = 0 must disable the check; violations: {violations:?}"
    );
}

#[test]
fn test_check_sarif_data_model_rules_carry_fixture_lines() {
    // §7.3: SARIF 2.1.0 output for the §7.1 fixture carries the
    // excessive_fields and primitive_field rule ids with the fixture line
    // regions (ViolationReport.line threaded into region.startLine).
    let (dir, staged) = data_model_fixture_dir("rust_data_model_sample.rs");

    let output = check_in(&dir, &staged)
        .arg("--format")
        .arg("sarif")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let sarif: serde_json::Value =
        serde_json::from_slice(&output.stdout).unwrap_or_else(|e| panic!("stdout should be valid SARIF json: {e:?}"));
    assert_eq!(sarif["version"].as_str(), Some("2.1.0"));

    let rules = sarif["runs"][0]["tool"]["driver"]["rules"]
        .as_array()
        .unwrap_or_else(|| panic!("rules array"));
    let rule_ids: Vec<&str> = rules.iter().filter_map(|r| r["id"].as_str()).collect();
    assert!(
        rule_ids.contains(&"excessive_fields"),
        "rule ids must include excessive_fields: {rule_ids:?}"
    );
    assert!(
        rule_ids.contains(&"primitive_field"),
        "rule ids must include primitive_field: {rule_ids:?}"
    );

    let results = sarif["runs"][0]["results"]
        .as_array()
        .unwrap_or_else(|| panic!("results array"));
    let line_for = |rule_id: &str| -> Option<i64> {
        results
            .iter()
            .find(|r| r["ruleId"] == rule_id)
            .and_then(|r| r["locations"][0]["physicalLocation"]["region"]["startLine"].as_i64())
    };
    assert_eq!(line_for("excessive_fields"), Some(6));
    assert_eq!(line_for("primitive_field"), Some(8));
}

#[test]
fn test_check_r_without_type_captures_stays_silent() {
    // "Language without type captures is silent": R ships zero type
    // captures, so its module must parse cleanly with no data-model
    // violations and no diagnostics.
    let dir = tempdir();
    let staged = stage_data_model_fixture(&dir, "r_no_types_sample.R");

    let output = check_in(&dir, &staged)
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
    let file = &json["data"]["files"][0];
    let diagnostics = file["diagnostics"].as_array().unwrap_or_else(|| panic!("diagnostics array"));
    assert!(
        diagnostics.is_empty(),
        "R fixture must parse without diagnostics: {diagnostics:?}"
    );
    let violations = file["file_violations"]
        .as_array()
        .unwrap_or_else(|| panic!("file_violations array"));
    assert!(
        violations
            .iter()
            .all(|v| v["metric"] != "excessive_fields" && v["metric"] != "primitive_field"),
        "R without type captures must produce no data-model violations: {violations:?}"
    );
}
