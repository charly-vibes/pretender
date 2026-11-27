//! Shared integration-test harness for pretender's CLI test suites.
//!
//! Common fixtures and `Command` builders used by the per-command
//! integration test files (`check_test.rs`, `report_test.rs`, ...).
//! Included via `mod common;` from each suite file; not its own test target.
//!
//! # Responsibilities
//! Spawn the built `pretender` binary with hermetic git config, build
//! per-subcommand command starters, and provide temp-dir/fixture staging
//! plus git repo seeding helpers.
//!
//! # Rationale
//! Extracted once from `tests/cli_test.rs` during the wave-3 per-command
//! split so each suite shares one harness instead of duplicating it;
//! bodies moved verbatim with expect/unwrap calls converted to the
//! sanctioned panic-on-failure forms. `Command` configuration is
//! concentrated in `build` so the file's mutable-binding ratio stays low.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Build a `Command` for the pretender binary: hermetic GIT_CONFIG_GLOBAL is
/// applied to every spawned process, then the caller's extra configuration
/// runs on the mutable handle. Single owner of the file's `let mut` binding.
fn build(configure: impl FnOnce(&mut Command)) -> Command {
    let mut cmd = Command::new(pretender_bin());
    cmd.env("GIT_CONFIG_GLOBAL", neutral_git_config_global());
    configure(&mut cmd);
    cmd
}

#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn git_init(dir: &Path) {
    let output = Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git init: {e:?}"));
    assert!(
        output.status.success(),
        "git init failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git config email: {e:?}"));
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git config name: {e:?}"));
}

// consumed by check_test (.4.1) and init_test (.4.3); the allow silences
// dead-code in the other test targets
#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn git_add(dir: &Path, path: &Path) {
    assert!(Command::new("git")
        .arg("add")
        .arg(path)
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git add: {e:?}"))
        .status
        .success());
}

// consumed by check_test (.4.1) and init_test (.4.3); the allow silences
// dead-code in the other test targets
#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn git_commit(dir: &Path, message: &str) {
    assert!(Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(dir)
        .output()
        .unwrap_or_else(|e| panic!("git commit: {e:?}"))
        .status
        .success());
}

pub fn pretender_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/debug/pretender")
}

/// Neutral, shared empty file used as GIT_CONFIG_GLOBAL for every spawned
/// pretender process: keeps core.hooksPath resolution (genesis multi-scope)
/// hermetic — tests must never see the developer machine's real global
/// config, nor write hooks into it.
pub fn neutral_git_config_global() -> &'static std::path::Path {
    static NEUTRAL: std::sync::LazyLock<std::path::PathBuf> = std::sync::LazyLock::new(|| {
        let p = std::env::temp_dir().join(format!(
            "pretender-test-empty-gitconfig-{}",
            std::process::id()
        ));
        std::fs::write(&p, "").unwrap_or_else(|e| panic!("write neutral gitconfig: {e:?}"));
        p
    });
    NEUTRAL.as_path()
}

/// Append `args` to a prepared command and hand it back (chaining helper).
fn with_args(mut cmd: Command, args: &[&str]) -> Command {
    cmd.args(args);
    cmd
}

/// Apply the tiered-mode advisory lease used by check call sites (GH #35).
/// Exit-policy tests override the mode afterwards.
fn tiered(mut cmd: Command) -> Command {
    cmd.arg("--mode")
        .arg("tiered")
        .arg("--advisory-until")
        .arg("2099-12-31")
        .env("NO_COLOR", "1");
    cmd
}

/// Run subcommand `name` from `dir` with color output suppressed.
fn subcommand_in(name: &str, dir: &Path) -> Command {
    build(|cmd| {
        cmd.arg(name).current_dir(dir).env("NO_COLOR", "1");
    })
}

pub fn pretender_cmd() -> Command {
    build(|_cmd| {})
}

pub fn source_fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tests/fixtures")
        .join(name)
}

// Copy a fixture into a fresh temp dir so role detection treats the file
// as the default `app` role rather than `test` (which would trip the
// stricter test thresholds against simple fixtures).
#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn stage_fixture(name: &str) -> (PathBuf, PathBuf) {
    let dir = tempdir();
    let dest = dir.join(name);
    std::fs::copy(source_fixture(name), &dest).unwrap_or_else(|e| panic!("copy fixture: {e:?}"));
    (dir, dest)
}

#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn write_temp_file(relative: &str, source: &str) -> (PathBuf, PathBuf) {
    let dir = tempdir();
    let dest = dir.join(relative);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|e| panic!("create parent dirs: {e:?}"));
    }
    std::fs::write(&dest, source).unwrap_or_else(|e| panic!("write temp source: {e:?}"));
    (dir, dest)
}

pub fn tempdir() -> PathBuf {
    // Atomic counter disambiguates parallel test threads that read the clock
    // within the same tick — pid+nanos alone collided, making `git init` fail
    // with 'File exists' when copying template hooks (seen 2026-10-01,
    // pretender-ivo).
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_else(|e| panic!("system clock before unix epoch: {e:?}"))
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("pretender-cli-{pid}-{nanos}-{seq}"));
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("create tempdir: {e:?}"));
    dir
}

pub fn check(path: &Path) -> Command {
    tiered(build(|cmd| {
        // Output-focused helper: carries a current advisory lease so call sites
        // keep advisory exit semantics (gate is now the default — pretender-1te).
        // Exit-policy tests set --mode explicitly.
        cmd.arg("check").arg(path);
    }))
}

#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn check_default(dir: &Path) -> Command {
    tiered(build(|cmd| {
        cmd.arg("check");
        cmd.current_dir(dir);
    }))
}

#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn check_in(dir: &Path, path: &Path) -> Command {
    let mut cmd = check(path);
    cmd.current_dir(dir);
    cmd
}

// consumed by report_format_test (.4.2); the allow silences dead-code in the
// other test targets
#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn report_in(dir: &Path) -> Command {
    subcommand_in("report", dir)
}

// consumed by tooling_test (.4.4); the allow silences dead-code in the other
// test targets
#[allow(dead_code)]
pub fn ci_generate_in(dir: &Path, provider: &str) -> Command {
    with_args(subcommand_in("ci", dir), &["generate", provider])
}

// consumed by init_test (.4.3); the allow silences dead-code in the other
// test targets
#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn hooks_in(dir: &Path, action: &str) -> Command {
    with_args(subcommand_in("hooks", dir), &[action])
}

// consumed by init_test (.4.3); the allow silences dead-code in the other
// test targets
#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn hooks_in_named(dir: &Path, action_and_hook: (&str, &str)) -> Command {
    let (action, hook) = action_and_hook;
    with_args(hooks_in(dir, action), &[hook])
}

// consumed by init_test (.4.3); the allow silences dead-code in the other
// test targets
#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn init_in(dir: &Path) -> Command {
    subcommand_in("init", dir)
}

/// Run the doctor subcommand from `dir` with color output suppressed and the
/// neutral global git config, plus any extra CLI arguments.
#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn doctor_in(dir: &Path, extra_args: &[&str]) -> Command {
    let mut cmd = pretender_cmd();
    cmd.arg("doctor")
        .args(extra_args)
        .current_dir(dir)
        .env("NO_COLOR", "1")
        .env("GIT_CONFIG_GLOBAL", neutral_git_config_global());
    cmd
}

/// The `metric` violations reported anywhere in a `check` envelope, as
/// `(path, actual)` pairs across file-level and unit-level violations.
#[allow(dead_code)] // shared harness: not every test target consumes every helper
pub fn metric_findings(envelope: &serde_json::Value, metric: &str) -> Vec<(String, f64)> {
    let files = envelope["data"]["files"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    files
        .iter()
        .flat_map(|file| file_findings(file, metric))
        .collect()
}

fn file_findings(file: &serde_json::Value, metric: &str) -> Vec<(String, f64)> {
    let path = file["path"].as_str().unwrap_or_default().to_string();
    let units = file["units"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let mut pairs: Vec<(String, f64)> = violation_actuals(&file["file_violations"], metric)
        .into_iter()
        .map(|actual| (path.clone(), actual))
        .collect();
    let unit_actuals: Vec<f64> = units
        .iter()
        .flat_map(|unit| violation_actuals(unit, metric))
        .collect();
    pairs.extend(
        unit_actuals
            .into_iter()
            .map(|actual| (path.clone(), actual)),
    );
    pairs
}

fn violation_actuals(violations: &serde_json::Value, metric: &str) -> Vec<f64> {
    violations
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .filter(|v| v["metric"].as_str() == Some(metric))
        .map(|v| v["actual"].as_f64().unwrap_or_default())
        .collect()
}
