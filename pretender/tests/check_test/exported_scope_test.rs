//! `exported_params` full-repo/scoped parity regressions
//! (pretender-jg9y gap 4).
//!
//! # Responsibilities
//! Pin that a full-repo `check` (walked from `.`) reports library-role
//! `exported_params` findings identically to a path-scoped run.
//!
//! # Rationale
//! Walked paths carry a `./` prefix, so role globs like `srclib/**` never
//! matched and the file fell back to the app role whose export rules are
//! off — exported findings went silent exactly when the gate ran whole.

use crate::common::*;
use std::path::Path;

fn write(path: &Path, content: &str) -> std::path::PathBuf {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("mkdir {}: {e:?}", path.display()));
    }
    std::fs::write(path, content).unwrap_or_else(|e| panic!("write {}: {e:?}", path.display()));
    path.to_path_buf()
}

/// Run `check --mode gate --format json` in `dir` and parse the envelope.
fn run_json(dir: &Path, extra: &[&str]) -> serde_json::Value {
    let output = pretender_cmd()
        .arg("check")
        .arg("--mode")
        .arg("gate")
        .arg("--format")
        .arg("json")
        .args(extra)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("JSON envelope: {e}\nstdout: {stdout}"))
}

/// True when some unit in the envelope reports the metric.
fn has_metric(envelope: &serde_json::Value, metric: &str) -> bool {
    envelope["data"]["files"]
        .as_array()
        .map(|files| {
            files.iter().any(|file| {
                let mut hit = file["file_violations"]
                    .as_array()
                    .map(|list| {
                        list.iter()
                            .any(|violation| violation["metric"].as_str() == Some(metric))
                    })
                    .unwrap_or(false);
                if let Some(units) = file["units"].as_array() {
                    hit = hit
                        || units.iter().any(|unit| {
                            unit["violations"]
                                .as_array()
                                .map(|list| {
                                    list.iter().any(|violation| {
                                        violation["metric"].as_str() == Some(metric)
                                    })
                                })
                                .unwrap_or(false)
                        });
                }
                hit
            })
        })
        .unwrap_or(false)
}

fn staged_library_repo() -> std::path::PathBuf {
    let dir = tempdir();
    write(
        &dir.join("pretender.toml"),
        "[roles.library]\npaths = [\"srclib/**\"]\n",
    );
    write(
        &dir.join("srclib/wide.rs"),
        "pub fn wide(a: u32, b: u32, c: u32, d: u32, e: u32) -> u32 {\n    a + b + c + d + e\n}\n",
    );
    dir
}

#[test]
fn exported_params_reports_in_full_repo_mode() {
    let dir = staged_library_repo();

    // No path arguments: the walk root is `.`, entries come back
    // `./`-prefixed — the exported finding must still be reported.
    let envelope = run_json(&dir, &[]);
    assert!(
        has_metric(&envelope, "exported_params"),
        "full-repo gate must report exported_params; envelope: {envelope}"
    );
}

#[test]
fn exported_params_reports_in_scoped_mode_for_parity() {
    let dir = staged_library_repo();

    let envelope = run_json(&dir, &["srclib"]);
    assert!(
        has_metric(&envelope, "exported_params"),
        "scoped run is the parity reference and must report exported_params; envelope: {envelope}"
    );
}
