//! Check-report output stages: writing the empty and full reports, the
//! gate summary line, `--adopt` persistence, and the final flush + verdict.
//!
//! # Responsibilities
//! Every stdout/file byte `pretender check` produces after the scan, plus
//! the adopt flow. Formatting lives in `crate::report`; gate decisions in
//! `super::gate`.
//!
//! # Rationale
//! Split from the check-pipeline driver (pretender-5zo.2.2) and then out of
//! `run/mod.rs` (pretender-5zo.15.12) along the orchestration/reporting
//! seam; bodies moved verbatim with stage helpers re-grouped so every
//! function stays within the structural limits.
use super::{load_gate_baseline, Stage};
use crate::check::gate::{
    decide_exit_code, violation_finding_ids, violation_findings, ViolationFinding, BASELINE_PATH,
};
use crate::cli::args::ReportFormat;
use crate::cli::infra::{color_enabled, open_report_sink, persist_last_check_report};
use crate::report::{
    write_human_report, write_json_report, write_sarif_report, CheckReport, HumanOpts,
};
use crate::CheckArgs;
use anyhow::{Context, Result};
use pretender::config::Config;
use pretender::history;
use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

/// Write the check report and gate summary, persist, and return the gate
/// verdict.
pub(super) fn write_outputs(
    stage: &Stage<'_>,
    lease_warning: Option<String>,
    sink: &mut dyn Write,
) -> Result<ExitCode> {
    let (args, report, config, format, current_ids) = stage;
    let (baselined, baseline_present) = load_gate_baseline(config, current_ids)?;
    let ctx = OutputCtx {
        config,
        format: *format,
        verbose: args.report.show_all,
        color: args.report.output.is_none() && color_enabled(),
    };
    write_check_report(sink, report, &ctx, lease_warning)?;
    if baseline_present && matches!(format, ReportFormat::Human) {
        write_gate_summary(sink, &baselined, current_ids, ctx.color)?;
    }
    persist_and_decide(report, config, sink, &baselined)
}

/// Presentation context for the check report: everything the writers need,
/// grouped so helpers stay within the parameter limit.
struct OutputCtx<'a> {
    config: &'a Config,
    format: ReportFormat,
    verbose: bool,
    color: bool,
}

/// Empty report for the nothing-staged/changed short-circuit.
fn empty_check_report() -> CheckReport {
    CheckReport {
        files: vec![],
        modules: Vec::new(),
        cycles: Vec::new(),
        test_findings: Vec::new(),
        history: None,
    }
}

/// Write the empty short-circuit report in the requested format and return
/// the resulting exit code.
pub(super) fn no_changes_exit(
    args: &CheckArgs,
    config: &Config,
    format: ReportFormat,
    lease_warning: Option<String>,
) -> Result<ExitCode> {
    let report = empty_check_report();
    write_no_changes_report(args, &report, format, lease_warning)?;
    Ok(decide_exit_code(
        &report,
        config.pretender.mode,
        config.pretender.advisory_until.as_deref(),
        &HashSet::new(),
    ))
}

/// The short-circuit output: human keeps the friendly one-liner;
/// JSON/SARIF consumers get a parseable empty report (pretender-xgt).
fn write_no_changes_report(
    args: &CheckArgs,
    report: &CheckReport,
    format: ReportFormat,
    lease_warning: Option<String>,
) -> Result<()> {
    if matches!(format, ReportFormat::Human) {
        print_no_changes_human(args.scan.staged);
        return Ok(());
    }
    let mut sink = open_report_sink(args.report.output.as_deref())?;
    match format {
        ReportFormat::Json => write_json_report(sink.as_mut(), report, lease_warning)?,
        ReportFormat::Sarif => write_sarif_report(sink.as_mut(), report)?,
        ReportFormat::Human => unreachable!("handled above"),
    }
    sink.flush().context("failed to flush report output")
}

/// The human-format short-circuit line.
fn print_no_changes_human(staged: bool) {
    if staged {
        println!("No staged files to check.");
    } else {
        println!("No changed files to check.");
    }
}

/// `--adopt`: persist the current scan's stable finding IDs as the gate
/// baseline and report what was adopted. The run still updates history, but
/// the exit code is success by definition — adoption is the decision.
pub(super) fn adopt_baseline(report: &CheckReport, format: ReportFormat) -> Result<ExitCode> {
    let findings = violation_findings(report);
    let current_ids = violation_finding_ids(report);
    let breakdown = finding_breakdown(&findings);
    let baseline = history::persist_baseline(Path::new(BASELINE_PATH), &current_ids)
        .context("failed to write gate baseline")?;
    if matches!(format, ReportFormat::Human) {
        println!(
            "\u{1b}[32m\u{2713}\u{1b}[0m adopted {} findings into {BASELINE_PATH} ({breakdown})",
            baseline.finding_ids.len()
        );
    }
    Ok(ExitCode::SUCCESS)
}

/// Rule-key → count summary of the findings being adopted.
fn finding_breakdown(findings: &[ViolationFinding]) -> String {
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for finding in findings {
        *kinds.entry(finding.rule_key.as_str()).or_default() += 1;
    }
    kinds
        .iter()
        .map(|(rule, count)| format!("{rule} {count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Write the report in the configured format.
fn write_check_report(
    sink: &mut dyn Write,
    report: &CheckReport,
    ctx: &OutputCtx<'_>,
    lease_warning: Option<String>,
) -> Result<()> {
    match ctx.format {
        ReportFormat::Human => write_human(sink, report, ctx),
        ReportFormat::Json => write_json_report(sink, report, lease_warning),
        ReportFormat::Sarif => write_sarif_report(sink, report),
    }
}

/// The human-format report with the presentation options from `ctx`.
fn write_human(sink: &mut dyn Write, report: &CheckReport, ctx: &OutputCtx<'_>) -> Result<()> {
    write_human_report(
        sink,
        report,
        &HumanOpts {
            color: ctx.color,
            bands: &ctx.config.bands,
            mode: ctx.config.pretender.mode,
            verbose: ctx.verbose,
        },
    )
}

/// Append the gate summary line when a baseline is in force (human format).
fn write_gate_summary(
    sink: &mut dyn Write,
    baselined: &HashSet<String>,
    current_ids: &[String],
    color: bool,
) -> Result<()> {
    let new_count = current_ids
        .iter()
        .filter(|id| !baselined.contains(*id))
        .collect::<HashSet<_>>()
        .len();
    let (cyan, reset) = gate_colors(color);
    writeln!(
        sink,
        "{cyan}\u{2192}{reset} gate: {} known (baselined), {new_count} new",
        baselined.len()
    )?;
    Ok(())
}

/// ANSI cyan/reset pair, empty when color is off.
fn gate_colors(color: bool) -> (&'static str, &'static str) {
    if color {
        ("\u{1b}[36m", "\u{1b}[0m")
    } else {
        ("", "")
    }
}

/// Persist the report for `pretender report`, flush the sink, and return
/// the gate verdict.
fn persist_and_decide(
    report: &CheckReport,
    config: &Config,
    sink: &mut dyn Write,
    baselined: &HashSet<String>,
) -> Result<ExitCode> {
    persist_last_check_report(report)?;
    sink.flush().context("failed to flush report output")?;
    Ok(decide_exit_code(
        report,
        config.pretender.mode,
        config.pretender.advisory_until.as_deref(),
        baselined,
    ))
}
