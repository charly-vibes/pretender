//! `check` unwraps-exclusion regressions (pretender-jg9y gap 2).
//!
//! # Responsibilities
//! Pin what the `unwraps` metric must not count for test-role files:
//! `.unwrap_or*` recovery calls, and `.expect(...)` sites inside
//! `#[cfg(test)]`-gated modules; plus the app-role control that keeps
//! counting them.
//!
//! # Rationale
//! Test-file violation counts were inflated by recovery calls and
//! double-gated test code; each exclusion here has a control proving it
//! does not over-silence.

use crate::common::*;
use std::path::Path;

fn fixture_source(name: &str) -> String {
    std::fs::read_to_string(source_fixture(name))
        .unwrap_or_else(|e| panic!("read fixture {name}: {e}"))
}

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

/// The unwraps violations reported anywhere in the envelope, as
/// `path actual` strings.
fn unwrap_findings(envelope: &serde_json::Value) -> Vec<(String, f64)> {
    metric_findings(envelope, "unwraps")
}

#[test]
fn unwrap_or_recovery_is_not_counted() {
    let dir = tempdir();
    // unwrap_max = 0 would disable the check; 1 keeps it live, so any
    // wrongly-counted unwrap_or site would fire.
    write(
        &dir.join("pretender.toml"),
        "[thresholds]\nunwrap_max = 1\n",
    );
    write(
        &dir.join("unwrap_or_only.rs"),
        &fixture_source("rust_unwrap_or_only.rs"),
    );

    let envelope = run_json(&dir, &[]);
    let findings = unwrap_findings(&envelope);
    assert!(
        findings.is_empty(),
        "unwrap_or recovery must not count as unwraps; findings: {findings:?}"
    );
}

#[test]
fn cfg_test_module_expects_are_excluded_for_test_role() {
    let dir = tempdir();
    // Six expects, all inside the #[cfg(test)] module; unwrap_max = 1
    // keeps the check live (0 would disable it), so any counted gated
    // site trips it.
    write(
        &dir.join("pretender.toml"),
        "[thresholds.test]\nunwrap_max = 1\n",
    );
    write(
        &dir.join("tests/cfg_expects_test.rs"),
        &fixture_source("rust_cfg_test_expects.rs"),
    );

    let envelope = run_json(&dir, &[]);
    let findings = unwrap_findings(&envelope);
    assert!(
        findings.is_empty(),
        "cfg(test)-gated expects in test-role files must be excluded; findings: {findings:?}"
    );
}

#[test]
fn cfg_test_module_expects_still_count_for_app_role() {
    let dir = tempdir();
    // Same fixture at the repo root: role app keeps counting gated
    // expects, so the test-role exclusion does not over-silence.
    write(
        &dir.join("cfg_expects.rs"),
        &fixture_source("rust_cfg_test_expects.rs"),
    );

    let envelope = run_json(&dir, &[]);
    let findings = unwrap_findings(&envelope);
    assert!(
        !findings.is_empty(),
        "app-role files keep counting cfg(test)-gated expects"
    );
}

#[test]
fn cfg_test_module_exclusion_keeps_outside_expects() {
    let dir = tempdir();
    write(
        &dir.join("pretender.toml"),
        "[thresholds.test]\nunwrap_max = 1\n",
    );
    // Two expects, both OUTSIDE any #[cfg(test)] module: they must keep
    // counting for the test-role file (2 > 1 fires).
    write(
        &dir.join("tests/mixed_expects_test.rs"),
        "fn outside_gate_a() {\n    let value: Result<u8, ()> = Ok(1);\n    let _ = value.expect(\"outside a\");\n}\n\nfn outside_gate_b() {\n    let value: Result<u8, ()> = Ok(1);\n    let _ = value.expect(\"outside b\");\n}\n",
    );

    let envelope = run_json(&dir, &[]);
    let findings = unwrap_findings(&envelope);
    assert_eq!(
        findings,
        vec![(String::from("./tests/mixed_expects_test.rs"), 2.0)],
        "outside-gate expects keep counting; findings: {findings:?}"
    );
}
