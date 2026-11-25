//! Check scan: file selection, per-file analysis passes, and the
//! test-report probe.
//!
//! Turns CLI path selection into the file list, runs the parallel
//! per-file analysis, merges external plugin and coupling findings, and
//! resolves the optional JUnit report for test-duration analysis.
//!
//! # Responsibilities
//! Everything between "have config" and "have a CheckReport": staged/diff
//! filtering, the gitignore-aware walk (via `super::collect`), the
//! analysis fan-out, and the `test_cmd` probe with its timeout handling.
//!
//! # Rationale
//! Split out of the check-pipeline driver (pretender-5zo.2.2) so the scan
//! stages and the report-writing stages stay under the structural file
//! and function limits; bodies moved verbatim, with the two oversized
//! functions split into verbatim-behaving helpers.
use super::analyze::analyze_path;
use super::collect::{apply_file_filter, collect_input_files_with_override};
use crate::{CheckArgs, CheckReport, CouplingViolationReport, FileReport, ModuleReport, get_parser};
use anyhow::{Context, Result};
use pretender::config::Config;
use pretender::model::Module;
use pretender::roles::RoleDetector;
use pretender::{coupling, external_plugin, git, test_report};
use rayon::prelude::*;
use std::path::PathBuf;

/// Collect and analyze the files to check. `Ok(None)` means a staged/diff
/// run found no allowed files — the caller short-circuits with a
/// format-aware empty report.
pub(super) fn scan_for_check(
    args: &CheckArgs,
    detector: &RoleDetector,
    config: &Config,
) -> Result<Option<CheckReport>> {
    let Some(files) = select_check_files(args, config)? else {
        return Ok(None);
    };
    warn_unsupported_paths(args, &files);
    let mut report = scan_files(&files, detector, config)?;
    run_test_duration_pass(args, &mut report, detector, config)?;
    Ok(Some(report))
}

/// Collect the files to check. `Ok(None)` means a staged/diff run found no
/// allowed files — the caller short-circuits with a format-aware empty
/// report.
fn select_check_files(args: &CheckArgs, config: &Config) -> Result<Option<Vec<PathBuf>>> {
    let files = if args.staged || args.diff_only || args.diff_base.is_some() {
        let cwd = std::env::current_dir().context("failed to get current directory")?;
        let allowed = if args.staged {
            git::staged_files(&cwd)?
        } else {
            let base = args.diff_base.as_deref().unwrap_or(&config.scope.diff_base);
            git::diff_base_files(&cwd, base)?
        };
        // Short-circuit: skip the full directory walk when nothing is staged/changed.
        // pretender-xgt: the skipped output must still be format-aware —
        // --format json/sarif consumers get a parseable empty report, not
        // human text. Human keeps the friendly one-liner.
        if allowed.is_empty() {
            return Ok(None);
        }
        let all = collect_input_files_with_override(&args.paths, config, args.scan_ignored)?;
        apply_file_filter(all, Some(&allowed), &cwd)
    } else {
        collect_input_files_with_override(&args.paths, config, args.scan_ignored)?
    };
    Ok(Some(files))
}

/// Warn when explicit paths produce no files matching any supported grammar.
/// This catches misconfigurations where new language backends are added but
/// pretender can't parse them, preventing false confidence from an empty
/// green check.
fn warn_unsupported_paths(args: &CheckArgs, files: &[PathBuf]) {
    let explicit_paths = !args.staged && !args.diff_only && args.diff_base.is_none();
    if explicit_paths && !files.iter().any(|f| get_parser(f).is_ok()) {
        let unsupported: Vec<String> = args
            .paths
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        eprintln!(
            "warning: no files matched a supported grammar in: {}",
            unsupported.join(", ")
        );
    }
}

/// Analyze every file and assemble the report with external and coupling
/// findings attached.
fn scan_files(files: &[PathBuf], detector: &RoleDetector, config: &Config) -> Result<CheckReport> {
    let (reports, modules) = analyze_files(files, detector, config)?;
    let mut report = CheckReport {
        files: reports,
        modules: Vec::new(),
        cycles: Vec::new(),
        test_findings: Vec::new(),
        history: None,
    };
    attach_external_findings(&mut report, files);
    attach_coupling(&mut report, &modules, config);
    Ok(report)
}

/// A module's import graph paired with its source path, kept together for
/// the coupling pass.
struct PathModule {
    path: PathBuf,
    module: Module,
}

/// Parse and analyze files in parallel, sorted by path, split into reports
/// and modules for coupling analysis.
fn analyze_files(
    files: &[PathBuf],
    detector: &RoleDetector,
    config: &Config,
) -> Result<(Vec<FileReport>, Vec<PathModule>)> {
    let mut reports_and_modules: Vec<(FileReport, Module)> = files
        .par_iter()
        .filter_map(|path| analyze_path(path, detector, config).transpose())
        .collect::<Result<_>>()?;
    reports_and_modules.sort_by(|a, b| a.0.path.cmp(&b.0.path));

    // Extract reports and modules for coupling analysis
    let mut reports: Vec<FileReport> = Vec::with_capacity(reports_and_modules.len());
    let mut modules: Vec<PathModule> = Vec::with_capacity(reports_and_modules.len());
    for (report, module) in reports_and_modules {
        let path = PathBuf::from(&report.path);
        reports.push(report);
        modules.push(PathModule { path, module });
    }
    Ok((reports, modules))
}

/// Merge external plugin findings into their file reports.
fn attach_external_findings(report: &mut CheckReport, files: &[PathBuf]) {
    let plugins = external_plugin::load_plugins(&external_plugin::default_metrics_dir());
    if !plugins.is_empty() {
        let mut external = external_plugin::run_plugins(&plugins, files);
        for file_report in &mut report.files {
            if let Some(findings) = external.remove(&file_report.path) {
                file_report.external_findings = findings;
            }
        }
    }
}

/// Run coupling analysis if thresholds are configured and attach module
/// violations and cycles to the report.
fn attach_coupling(report: &mut CheckReport, modules: &[PathModule], config: &Config) {
    let coupling_result = coupling::analyze(
        &modules
            .iter()
            .map(|pm| (pm.path.clone(), &pm.module))
            .collect::<Vec<_>>(),
        &config.thresholds.coupling,
    );
    report.modules = coupling_result
        .modules
        .iter()
        .map(|m| ModuleReport {
            path: m.path.clone(),
            coupling_violations: m
                .violations
                .iter()
                .map(|v| CouplingViolationReport {
                    metric: v.metric.clone(),
                    actual: v.actual,
                    limit: v.limit,
                })
                .collect(),
        })
        .collect();
    report.cycles = coupling_result
        .cycles
        .iter()
        .map(|c| c.participants.clone())
        .collect();
}

/// Attach test-duration findings when a test report is available.
fn run_test_duration_pass(
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
    report.test_findings =
        test_report::evaluate_duration(&timings, detector, config, &search_root);
    Ok(())
}

/// Resolve the test report path to use for duration analysis.
///
/// Priority:
/// 1. `--test-report` flag (explicit path from user)
/// 2. `--execute` flag: run `[execute] test_cmd` and use `test_report_path`
/// 3. Neither: returns `None` (no duration check)
fn resolve_test_report_path(args: &CheckArgs, config: &Config) -> Option<PathBuf> {
    if let Some(path) = &args.test_report {
        return Some(path.clone());
    }

    if !args.execute {
        return None;
    }

    let cmd = config.execute.test_cmd.as_ref()?;
    let report_path = config.execute.test_report_path.as_ref()?;
    let timeout = std::time::Duration::from_secs(config.execute.test_timeout_s as u64);

    eprintln!("Running test command: {}", cmd);
    if !run_test_cmd(cmd, timeout) {
        return None;
    }

    let report_path = PathBuf::from(report_path);
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
fn run_test_cmd(cmd: &str, timeout: std::time::Duration) -> bool {
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
fn wait_test_result(child: &mut std::process::Child, timeout: std::time::Duration) -> bool {
    match wait_with_timeout(child, timeout) {
        Ok(Some(status)) => {
            if !status.success() {
                eprintln!("warning: test_cmd exited with non-zero status ({})", status);
            }
            true
        }
        Ok(None) => {
            eprintln!(
                "warning: test_cmd timed out after {}s",
                timeout.as_secs()
            );
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
    timeout: std::time::Duration,
) -> std::io::Result<Option<std::process::ExitStatus>> {
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Some(status)),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    return Ok(None);
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            Err(e) => return Err(e),
        }
    }
}
