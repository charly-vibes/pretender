//! Check scan: file selection, per-file analysis passes, and coupling.
//!
//! Turns CLI path selection into the file list, runs the parallel
//! per-file analysis, and merges external plugin and coupling findings.
//!
//! # Responsibilities
//! Everything between "have config" and "have a CheckReport": staged/diff
//! filtering, the gitignore-aware walk (via `super::collect`), and the
//! analysis fan-out. The `test_cmd` probe lives in `super::scan::test_cmd`;
//! the parser registry in `super::scan::parsers`.
//!
//! # Rationale
//! Split out of the check-pipeline driver (pretender-5zo.2.2) and then into
//! a module dir along the scan/process/parsers seams
//! (pretender-5zo.15.12); bodies moved verbatim, with oversized functions
//! split into verbatim-behaving helpers.
use super::analyze::analyze_path;
use super::collect::{apply_file_filter, collect_input_files_with_override};
use crate::report::{CheckReport, CouplingViolationReport, FileReport, ModuleReport};
use crate::CheckArgs;
use anyhow::{Context, Result};
use pretender::config::Config;
use pretender::model::Module;
use pretender::role_resolution::RoleDetector;
use pretender::{coupling, external_plugin, git};
use rayon::prelude::*;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(crate) use parsers::get_parser;

pub(crate) mod parsers;
mod test_cmd;

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
    test_cmd::run_test_duration_pass(args, &mut report, detector, config)?;
    Ok(Some(report))
}

/// Collect the files to check. `Ok(None)` means a staged/diff run found no
/// allowed files — the caller short-circuits with a format-aware empty
/// report (pretender-xgt: the skipped output is still format-aware —
/// --format json/sarif consumers get a parseable empty report, not human
/// text; human keeps the friendly one-liner).
fn select_check_files(args: &CheckArgs, config: &Config) -> Result<Option<Vec<PathBuf>>> {
    if !args.scan.staged && !args.scan.diff_only && args.scan.diff_base.is_none() {
        return collect_input_files_with_override(&args.scan.paths, config, args.scan.scan_ignored)
            .map(Some);
    }
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let allowed = allowed_files(args, &cwd, config)?;
    if allowed.is_empty() {
        return Ok(None);
    }
    let all = collect_input_files_with_override(&args.scan.paths, config, args.scan.scan_ignored)?;
    Ok(Some(apply_file_filter(all, Some(&allowed), &cwd)))
}

/// Staged or diff-base file list for filtered runs.
fn allowed_files(args: &CheckArgs, cwd: &Path, config: &Config) -> Result<HashSet<PathBuf>> {
    if args.scan.staged {
        git::staged_files(cwd)
    } else {
        let base = args
            .scan
            .diff_base
            .as_deref()
            .unwrap_or(&config.scope.diff_base);
        git::diff_base_files(cwd, base)
    }
}

/// Warn when explicit paths produce no files matching any supported grammar.
/// This catches misconfigurations where new language backends are added but
/// pretender can't parse them, preventing false confidence from an empty
/// green check.
fn warn_unsupported_paths(args: &CheckArgs, files: &[PathBuf]) {
    let explicit_paths = !args.scan.staged && !args.scan.diff_only && args.scan.diff_base.is_none();
    if explicit_paths && !files.iter().any(|f| get_parser(f).is_ok()) {
        let unsupported: Vec<String> = args
            .scan
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
    let (reports, modules) = split_reports_and_modules(reports_and_modules);
    Ok((reports, modules))
}

/// Split parsed results into reports and path-paired modules for coupling.
fn split_reports_and_modules(
    pairs: Vec<(FileReport, Module)>,
) -> (Vec<FileReport>, Vec<PathModule>) {
    pairs
        .into_iter()
        .map(|(report, module)| {
            let path = PathBuf::from(&report.path);
            (report, PathModule { path, module })
        })
        .unzip()
}

/// Merge external plugin findings into their file reports.
fn attach_external_findings(report: &mut CheckReport, files: &[PathBuf]) {
    let plugins = external_plugin::load_plugins(&external_plugin::default_metrics_dir());
    if plugins.is_empty() {
        return;
    }
    let mut external = external_plugin::run_plugins(&plugins, files);
    for file_report in &mut report.files {
        if let Some(findings) = external.remove(&file_report.path) {
            file_report.external_findings = findings;
        }
    }
}

/// Run coupling analysis if thresholds are configured and attach module
/// violations and cycles to the report.
fn attach_coupling(report: &mut CheckReport, modules: &[PathModule], config: &Config) {
    let result = coupling::analyze(&module_pairs(modules), &config.thresholds.coupling);
    report.modules = result
        .modules
        .iter()
        .map(|m| ModuleReport {
            path: m.path.clone(),
            coupling_violations: m.violations.iter().map(coupling_violation).collect(),
        })
        .collect();
    report.cycles = result
        .cycles
        .iter()
        .map(|c| c.participants.clone())
        .collect();
}

/// `(path, module)` pairs for the coupling analyzer.
fn module_pairs(modules: &[PathModule]) -> Vec<(PathBuf, &Module)> {
    modules
        .iter()
        .map(|pm| (pm.path.clone(), &pm.module))
        .collect()
}

/// The report view of one coupling violation.
fn coupling_violation(v: &coupling::CouplingViolation) -> CouplingViolationReport {
    CouplingViolationReport {
        metric: v.metric.clone(),
        actual: v.actual,
        limit: v.limit,
    }
}
