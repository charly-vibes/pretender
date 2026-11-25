//! Check-pipeline driver: the `check` subcommand's executable stage.
//!
//! Prepares config and lease, delegates file selection and analysis to
//! `super::scan`, handles adoption and the gate baseline, writes the
//! report in the requested format, and returns the exit code.
//!
//! # Responsibilities
//! Orchestrate `CheckArgs::run` as a chain of ≤30-line stage helpers;
//! keep decision logic in `super::gate`, scan stages in `super::scan`,
//! and file collection in `super::collect`.
//!
//! # Rationale
//! The original 278-line `run` (abc 140) exceeded every structural limit;
//! decomposed while moving so behavior, output bytes, and side-effect
//! ordering stay identical (pretender-5zo.2.2).
use super::gate::{
    decide_exit_code, lease_diagnostic, lease_fail_closed, violation_finding_ids,
    violation_findings, BASELINE_PATH,
};
use super::scan::scan_for_check;
use crate::cli::args::ReportFormat;
use crate::cli::infra::{
    color_enabled, load_config, open_report_sink, persist_last_check_report, warn_if_no_config,
};
use crate::cli::Executable;
use crate::events::{emit_history_events, EmitOpts};
use crate::report::{
    write_human_report, write_json_report, write_sarif_report, CheckReport, HumanOpts,
};
use crate::CheckArgs;
use anyhow::{Context, Result};
use pretender::config::{Config, Mode};
use pretender::history;
use pretender::role_resolution::RoleDetector;
use std::collections::{BTreeMap, HashSet};
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

impl Executable for CheckArgs {
    fn run(&self) -> Result<ExitCode> {
        warn_if_no_config();
        let (config, lease_warning) = prepare_check_config(self)?;
        let format = resolve_check_format(self.format);
        let detector = RoleDetector::new(&config).context("failed to initialize role detector")?;

        let Some(mut report) = scan_for_check(self, &detector, &config)? else {
            return no_changes_exit(self, &config, format, lease_warning);
        };
        let current_ids = violation_finding_ids(&report);
        let writing_to_stdout = self.output.is_none();
        let mut sink = attach_history_open_sink(self, &mut report, &config, format)?;

        if self.adopt {
            return adopt_baseline(&report, format);
        }
        let (baselined, baseline_present) = load_gate_baseline(&config, &current_ids)?;
        let ctx = OutputCtx {
            config: &config,
            format,
            verbose: self.show_all,
            color: writing_to_stdout && color_enabled(),
        };
        write_check_report(sink.as_mut(), &report, &ctx, lease_warning)?;
        if baseline_present && matches!(format, ReportFormat::Human) {
            write_gate_summary(sink.as_mut(), &baselined, &current_ids, ctx.color)?;
        }
        persist_and_decide(&report, &config, sink.as_mut(), &baselined)
    }
}

/// Presentation context for the check report: everything the writers need,
/// grouped so helpers stay within the parameter limit.
struct OutputCtx<'a> {
    config: &'a Config,
    format: ReportFormat,
    verbose: bool,
    color: bool,
}

/// Apply CLI overrides to the loaded config and evaluate the advisory-lease
/// fail-closed condition, printing the diagnostic to stderr when it trips.
fn prepare_check_config(args: &CheckArgs) -> Result<(Config, Option<String>)> {
    let mut config = load_config()?;
    if let Some(mode) = args.mode {
        config.pretender.mode = mode.into();
    }
    // CLI lease overrides config lease (same semantics either way)
    if args.advisory_until.is_some() {
        config.pretender.advisory_until = args.advisory_until.clone();
    }

    // Advisory-lease fail-closed (GH #35): when tiered/guidance run without
    // a current lease the check still fails, but the user must be told why.
    // The diagnostic goes to stderr so stdout stays a parseable report.
    let lease_failed = lease_fail_closed(
        config.pretender.mode,
        config.pretender.advisory_until.as_deref(),
    );
    let lease_warning = lease_failed.then(|| lease_diagnostic(config.pretender.mode));
    if let Some(msg) = &lease_warning {
        eprintln!("error: {msg}");
    }
    Ok((config, lease_warning))
}

/// Resolve the report format, honoring the global `--json` / `--human`
/// flags over the check-specific default.
fn resolve_check_format(default: ReportFormat) -> ReportFormat {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--json" || a == "-j") {
        ReportFormat::Json
    } else if args.iter().any(|a| a == "--human") {
        ReportFormat::Human
    } else {
        default
    }
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
fn no_changes_exit(
    args: &CheckArgs,
    config: &Config,
    format: ReportFormat,
    lease_warning: Option<String>,
) -> Result<ExitCode> {
    let report = empty_check_report();
    match format {
        ReportFormat::Human if args.staged => println!("No staged files to check."),
        ReportFormat::Human => println!("No changed files to check."),
        ReportFormat::Json | ReportFormat::Sarif => {
            let mut sink = open_report_sink(args.output.as_deref())?;
            match format {
                ReportFormat::Json => write_json_report(sink.as_mut(), &report, lease_warning)?,
                ReportFormat::Sarif => write_sarif_report(sink.as_mut(), &report)?,
                ReportFormat::Human => unreachable!("handled above"),
            }
            sink.flush().context("failed to flush report output")?;
        }
    }
    Ok(decide_exit_code(
        &report,
        config.pretender.mode,
        config.pretender.advisory_until.as_deref(),
        &HashSet::new(),
    ))
}

/// Open the report sink and attach history events so JSON format includes
/// them.
fn attach_history_open_sink(
    args: &CheckArgs,
    report: &mut CheckReport,
    config: &Config,
    format: ReportFormat,
) -> Result<Box<dyn Write>> {
    let mut sink = open_report_sink(args.output.as_deref())?;
    emit_history_events(
        report,
        config,
        &EmitOpts {
            format,
            stdout: args.output.is_none(),
        },
        sink.as_mut(),
    );
    Ok(sink)
}

/// `--adopt`: persist the current scan's stable finding IDs as the gate
/// baseline and report what was adopted. The run still updates history, but
/// the exit code is success by definition — adoption is the decision.
fn adopt_baseline(report: &CheckReport, format: ReportFormat) -> Result<ExitCode> {
    let findings = violation_findings(report);
    let current_ids = violation_finding_ids(report);
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    for finding in &findings {
        *kinds.entry(finding.rule_key.as_str()).or_default() += 1;
    }
    let breakdown = kinds
        .iter()
        .map(|(rule, count)| format!("{rule} {count}"))
        .collect::<Vec<_>>()
        .join(", ");
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

/// Known (adopted) finding IDs for gate mode. No baseline → every violation
/// gates (fail-closed preserved). A corrupt baseline errors out — it must
/// not silently degrade to "no baseline".
fn load_gate_baseline(config: &Config, current_ids: &[String]) -> Result<(HashSet<String>, bool)> {
    if matches!(config.pretender.mode, Mode::Gate) {
        match history::load_baseline(Path::new(BASELINE_PATH)) {
            Ok(Some(baseline)) => {
                let known: HashSet<String> = current_ids
                    .iter()
                    .filter(|id| baseline.finding_ids.contains(*id))
                    .cloned()
                    .collect();
                Ok((known, true))
            }
            Ok(None) => Ok((HashSet::new(), false)),
            Err(err) => Err(err.context("gate baseline unreadable — refusing to gate")),
        }
    } else {
        Ok((HashSet::new(), false))
    }
}

/// Write the report in the configured format.
fn write_check_report(
    sink: &mut dyn Write,
    report: &CheckReport,
    ctx: &OutputCtx<'_>,
    lease_warning: Option<String>,
) -> Result<()> {
    match ctx.format {
        ReportFormat::Human => write_human_report(
            sink,
            report,
            &HumanOpts {
                color: ctx.color,
                bands: &ctx.config.bands,
                mode: ctx.config.pretender.mode,
                verbose: ctx.verbose,
            },
        )?,
        ReportFormat::Json => write_json_report(sink, report, lease_warning)?,
        ReportFormat::Sarif => write_sarif_report(sink, report)?,
    }
    Ok(())
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
    let (cyan, reset) = if color {
        ("\u{1b}[36m", "\u{1b}[0m")
    } else {
        ("", "")
    };
    writeln!(
        sink,
        "{cyan}\u{2192}{reset} gate: {} known (baselined), {new_count} new",
        baselined.len()
    )?;
    Ok(())
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
