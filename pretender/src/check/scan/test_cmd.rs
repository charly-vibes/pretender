//! The `test_cmd` probe: optional test-command execution and JUnit report
//! resolution for the test-duration pass.
//!
//! # Responsibilities
//! Resolve the test report path (`--test-report` flag or `--execute`
//! running `[execute] test_cmd`), spawn the command with inherited output,
//! and enforce its timeout. Each failure mode is reported on stderr and
//! degrades to "no duration check".
//!
//! # Rationale
//! Split from the scan module (pretender-5zo.15.12); bodies moved verbatim
//! with the path-resolution and process-wait stages split into
//! verbatim-behaving helpers.
use crate::report::CheckReport;
use crate::CheckArgs;
use anyhow::{Context, Result};
use pretender::config::Config;
use pretender::role_resolution::RoleDetector;
use pretender::test_report;
use std::time::Duration;

/// Attach test-duration findings when a test report is available.
pub(super) fn run_test_duration_pass(
    args: &CheckArgs,
    report: &mut CheckReport,
    detector: &RoleDetector,
    config: &Config,
) -> Result<()> {
    // ── Test duration analysis ──────────────────────────────────
    let Some(report_path) = resolve_test_report_path(args, config) else {
        return Ok(());
    };
    let timings = test_report::parse_junit(&report_path, config.execute.test_time_unit)
        .context("failed to parse JUnit XML report")?;
    let search_root = std::env::current_dir().context("failed to get current directory")?;
    report.test_findings = test_report::evaluate_duration(&timings, detector, config, &search_root);
    Ok(())
}

/// Resolve the test report path to use for duration analysis.
///
/// Priority:
/// 1. `--test-report` flag (explicit path from user)
/// 2. `--execute` flag: run `[execute] test_cmd` and use `test_report_path`
/// 3. Neither: returns `None` (no duration check)
fn resolve_test_report_path(args: &CheckArgs, config: &Config) -> Option<std::path::PathBuf> {
    if let Some(path) = &args.test_report {
        return Some(path.clone());
    }
    if !args.execute {
        return None;
    }
    resolve_test_report_from_cmd(config)
}

/// `--execute`: run `[execute] test_cmd` and use `test_report_path`,
/// warning when the command fails or the report never appears (each
/// failure already reported on stderr).
fn resolve_test_report_from_cmd(config: &Config) -> Option<std::path::PathBuf> {
    let cmd = config.execute.test_cmd.as_ref()?;
    let report_path = config.execute.test_report_path.as_ref()?;
    let timeout = Duration::from_secs(config.execute.test_timeout_s as u64);
    eprintln!("Running test command: {}", cmd);
    if !run_test_cmd(cmd, timeout) {
        return None;
    }
    let report_path = std::path::PathBuf::from(report_path);
    if report_path.exists() {
        return Some(report_path);
    }
    eprintln!(
        "warning: test_report_path '{}' not found after test_cmd execution",
        report_path.display()
    );
    None
}

/// Spawn `test_cmd` and wait for it. Returns false when the command could
/// not run, timed out, or failed — each already reported on stderr.
fn run_test_cmd(cmd: &str, timeout: Duration) -> bool {
    let mut child = match spawn_test_cmd(cmd) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("warning: failed to spawn test_cmd '{}': {}", cmd, e);
            return false;
        }
    };
    wait_test_result(&mut child, timeout)
}

/// Spawn `test_cmd` with inherited output streams.
fn spawn_test_cmd(cmd: &str) -> std::io::Result<std::process::Child> {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
}

/// Wait for a spawned `test_cmd`. Returns false when it timed out, exited
/// non-zero, or failed — each already reported on stderr.
fn wait_test_result(child: &mut std::process::Child, timeout: Duration) -> bool {
    match wait_with_timeout(child, timeout) {
        Ok(Some(status)) => {
            if !status.success() {
                eprintln!("warning: test_cmd exited with non-zero status ({})", status);
            }
            true
        }
        Ok(None) => {
            eprintln!("warning: test_cmd timed out after {}s", timeout.as_secs());
            let _ = child.kill();
            let _ = child.wait();
            false
        }
        Err(e) => {
            eprintln!("warning: test_cmd failed: {}", e);
            false
        }
    }
}

/// Wait for a child process with a timeout. Returns `Ok(Some(status))`
/// if the child exited, `Ok(None)` if it timed out, or `Err` on system error.
fn wait_with_timeout(
    child: &mut std::process::Child,
    timeout: Duration,
) -> std::io::Result<Option<std::process::ExitStatus>> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Some(status)),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    return Ok(None);
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => return Err(e),
        }
    }
}
