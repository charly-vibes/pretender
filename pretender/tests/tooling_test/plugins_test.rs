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
use std::path::Path;

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
///
/// Runs `check --format json` with the external-plugin metrics dir pointing
/// at `metrics_dir`, asserting success and a valid envelope; returns the JSON.
fn run_ruff_check_json(metrics_dir: &Path) -> serde_json::Value {
    let (_dir, staged) = stage_fixture("python_simple.py");

    let output = check(&staged)
        .arg("--format")
        .arg("json")
        .env("PRETENDER_METRICS_DIR", metrics_dir)
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| panic!("valid json output: {e:?}"))
}

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

    let json = run_ruff_check_json(&metrics_dir);

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

/// Every builtin manifest parses via the plugin contract and stays in sync
/// with the registry record set (fvo9 design step 6): the manifests are
/// load-bearing — `WeightMaps` reads branches and smell_weights on every
/// parse — so this pins the required fields and the record/manifest
/// extension agreement for all 13 language dirs (TS/TSX share one).
#[test]
fn every_builtin_manifest_parses_and_matches_the_registry() {
    use pretender::plugin::PluginManifest;
    use pretender::registry;

    let mut pinned: Vec<pretender::model::Language> = Vec::new();
    for record in registry::REGISTRY.iter() {
        if pinned.contains(&record.language) {
            continue; // TS/TSX dual records share the typescript manifest
        }
        pinned.push(record.language.clone());

        let manifest = PluginManifest::from_toml(record.manifest)
            .unwrap_or_else(|e| panic!("builtin manifest must parse: {e:?}"));

        assert!(!manifest.name.is_empty(), "name is the language id");
        assert!(!manifest.display_name.is_empty(), "display_name renders");
        assert!(
            manifest
                .tree_sitter
                .source
                .starts_with("builtin:tree-sitter-"),
            "builtin manifests bind a builtin grammar, got {}",
            manifest.tree_sitter.source
        );
        assert!(
            !manifest.tree_sitter.rev.is_empty(),
            "grammar rev is pinned"
        );
        assert_eq!(manifest.query, "metrics.scm", "builtin query file name");

        let mut manifest_exts = manifest.extensions.clone();
        manifest_exts.sort();
        let mut record_exts: Vec<String> = registry::REGISTRY
            .iter()
            .filter(|r| r.language == record.language)
            .flat_map(|r| r.extensions.iter().map(|e| format!(".{e}")))
            .collect();
        record_exts.sort();
        record_exts.dedup();
        assert_eq!(
            manifest_exts, record_exts,
            "manifest and registry extension sets must agree"
        );

        assert!(
            !manifest.metric_sections.branches.is_empty(),
            "branch weights drive cognitive/cyclomatic"
        );
        assert!(
            !manifest.metric_sections.smell_weights.is_empty(),
            "smell weights drive score"
        );
        assert!(
            !manifest.metric_sections.assertions.patterns.is_empty(),
            "assertion patterns drive the assertion count"
        );
    }
    assert_eq!(pinned.len(), 13, "13 builtin language dirs pinned");
}

/// Spec pin (universal-code-model / clojure-readme-declares-unsupported-
/// metrics, pretender-1v8k.14): each shipped adapter README must carry the
/// Known unsupported metrics section and separate not-implemented-yet from
/// grammar-unsupported, so a missing metric never reads as "supported and
/// clean".
#[test]
fn test_clojure_readme_declares_unsupported_metrics() {
    let readme = include_str!("../../languages/clojure/README.md").to_lowercase();
    assert!(
        readme.contains("## known unsupported metrics"),
        "clojure README must declare its unsupported metrics"
    );
    assert!(
        readme.contains("not supported by the grammar") && readme.contains("not implemented yet"),
        "unsupported metrics must be classified with both reasons"
    );
}
