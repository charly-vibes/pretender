//! External-plugin execution and CI workflow generation tests.
//!
//! # Responsibilities
//! Pin the ruff external-plugin JSON finding mapping via
//! `PRETENDER_METRICS_DIR`, and `ci generate`'s GitHub workflow output plus
//! its non-GitHub stub behavior.
//!
//! # Rationale
//! Bodies moved verbatim from `tests/cli_test.rs` (wave-3 split) with
//! expect calls converted to the sanctioned panic-on-failure forms.

use crate::common::*;

const RUFF_PLUGIN_TOML: &str = r#"
name = "ruff"
extensions = [".py"]
command = ["ruff", "check", "--output-format=json", "--select=E501", "{files}"]
parser = "json"

[mapping]
path = "filename"
line = "location.row"
message = "message"
code = "code"
"#;

#[test]
fn test_ci_generate_github_writes_workflow() {
    let dir = tempdir();

    let output = ci_generate_in(&dir, "github")
        .output()
        .unwrap_or_else(|e| panic!("run ci generator: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let workflow = std::fs::read_to_string(dir.join(".github/workflows/pretender.yml"))
        .unwrap_or_else(|e| panic!("workflow exists: {e:?}"));
    assert!(
        workflow.starts_with("name: Pretender"),
        "workflow: {workflow}"
    );
    assert!(
        workflow.contains("uses: actions/checkout@v4"),
        "workflow: {workflow}"
    );
    assert!(
        workflow.contains("uses: dtolnay/rust-toolchain@stable"),
        "workflow: {workflow}"
    );
    assert!(
        workflow.contains(
            "cargo install --git https://github.com/charly-vibes/pretender --locked pretender"
        ),
        "workflow: {workflow}"
    );
    assert!(
        workflow.contains("pretender check . --format=sarif --output=pretender.sarif"),
        "workflow: {workflow}"
    );
    assert!(
        workflow.contains("uses: github/codeql-action/upload-sarif@v3"),
        "workflow: {workflow}"
    );
    assert!(
        workflow.contains("pretender report --format=markdown >> $GITHUB_STEP_SUMMARY"),
        "workflow: {workflow}"
    );
}

#[test]
fn test_ci_generate_non_github_stays_stubbed() {
    let dir = tempdir();

    let output = ci_generate_in(&dir, "gitlab")
        .output()
        .unwrap_or_else(|e| panic!("run ci generator: {e:?}"));

    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not yet implemented"), "stderr: {stderr}");
}

/// The python_simple.py fixture has a 112-char comment on line 1, which triggers
/// E501 (line too long > 79) when ruff is run with --select=E501.
#[test]
fn test_external_plugin_ruff_json_findings() {
    if std::process::Command::new("ruff")
        .arg("--version")
        .output()
        .is_err()
    {
        eprintln!("skipping: ruff not in PATH");
        return;
    }

    let metrics_dir = tempdir();
    std::fs::write(metrics_dir.join("ruff.toml"), RUFF_PLUGIN_TOML)
        .unwrap_or_else(|e| panic!("write ruff.toml: {e:?}"));

    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged)
        .arg("--format")
        .arg("json")
        .env("PRETENDER_METRICS_DIR", &metrics_dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|e| panic!("valid json output: {e:?}"));

    let files = json["data"]["files"]
        .as_array()
        .unwrap_or_else(|| panic!("files array"));
    assert_eq!(files.len(), 1);
    let external = files[0]["external_findings"]
        .as_array()
        .unwrap_or_else(|| panic!("external_findings present when ruff fires"));
    assert!(
        !external.is_empty(),
        "expected at least one ruff finding; full output: {json}"
    );
    assert_eq!(
        external[0]["source"].as_str(),
        Some("ruff"),
        "finding source should be 'ruff'"
    );
    assert_eq!(
        external[0]["code"].as_str(),
        Some("E501"),
        "finding code should be 'E501'"
    );
}
