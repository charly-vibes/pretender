//! Check-pipeline driver: the `check` subcommand's executable stage.
//!
//! Prepares config and lease, delegates file selection and analysis to
//! `super::scan`, handles adoption and the gate baseline, and returns the
//! exit code.
//!
//! # Responsibilities
//! Orchestrate `CheckArgs::run` as a chain of ≤20-line stage helpers;
//! keep decision logic in `super::gate`, scan stages in `super::scan`,
//! file collection in `super::collect`, and report writing in
//! `super::run::report`.
//!
//! # Rationale
//! The original 278-line `run` (abc 140) exceeded every structural limit;
//! decomposed while moving so behavior, output bytes, and side-effect
//! ordering stay identical (pretender-5zo.2.2). Split into `mod.rs` plus
//! `report.rs` along the orchestration/reporting seam
//! (pretender-5zo.15.12).
use self::report::{adopt_baseline, no_changes_exit, write_outputs};
use super::gate::{lease_diagnostic, lease_fail_closed, violation_finding_ids, BASELINE_PATH};
use super::scan::scan_for_check;
use crate::cli::args::ReportFormat;
use crate::cli::infra::{load_config, open_report_sink, warn_if_no_config};
use crate::cli::Executable;
use crate::events::{emit_history_events, EmitOpts};
use crate::report::CheckReport;
use crate::CheckArgs;
use anyhow::{Context, Result};
use pretender::config::{Config, Mode};
use pretender::history;
use pretender::role_resolution::RoleDetector;
use std::collections::HashSet;
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

mod report;

impl Executable for CheckArgs {
    fn run(&self) -> Result<ExitCode> {
        warn_if_no_config();
        let (config, lease_warning) = prepare_check_config(self)?;
        let format = resolve_check_format(self.report.format);
        let detector = RoleDetector::new(&config).context("failed to initialize role detector")?;

        let Some(mut report) = scan_for_check(self, &detector, &config)? else {
            return no_changes_exit(self, &config, format, lease_warning);
        };
        let current_ids = violation_finding_ids(&report);
        let mut sink = attach_history_open_sink(self, &mut report, &config, format)?;

        if self.adopt {
            return adopt_baseline(&report, format);
        }
        let stage: Stage<'_> = (self, &report, &config, format, &current_ids);
        write_outputs(&stage, lease_warning, sink.as_mut())
    }
}

/// Everything the output stage needs after a successful scan: the CLI args,
/// the finished report, the effective config, the resolved format, and the
/// scan's stable finding IDs. One tuple so `write_outputs` stays within the
/// parameter limit.
pub(super) type Stage<'a> = (
    &'a CheckArgs,
    &'a CheckReport,
    &'a Config,
    ReportFormat,
    &'a [String],
);

/// Apply CLI overrides to the loaded config and evaluate the advisory-lease
/// fail-closed condition, printing the diagnostic to stderr when it trips.
fn prepare_check_config(args: &CheckArgs) -> Result<(Config, Option<String>)> {
    let mut config = load_config()?;
    if let Some(mode) = args.mode {
        config.pretender.mode = mode.into();
    }
    if args.advisory_until.is_some() {
        config.pretender.advisory_until = args.advisory_until.clone();
    }
    let lease = lease_warning(&config);
    Ok((config, lease))
}

/// Advisory-lease fail-closed diagnostic for the loaded config (GH #35):
/// when tiered/guidance run without a current lease the check still fails,
/// but the user must be told why. The diagnostic goes to stderr so stdout
/// stays a parseable report.
fn lease_warning(config: &Config) -> Option<String> {
    let lease_failed = lease_fail_closed(
        config.pretender.mode,
        config.pretender.advisory_until.as_deref(),
    );
    let warning = lease_failed.then(|| lease_diagnostic(config.pretender.mode));
    if let Some(msg) = &warning {
        eprintln!("error: {msg}");
    }
    warning
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

/// Open the report sink and attach history events so JSON format includes
/// them.
fn attach_history_open_sink(
    args: &CheckArgs,
    report: &mut CheckReport,
    config: &Config,
    format: ReportFormat,
) -> Result<Box<dyn Write>> {
    let mut sink = open_report_sink(args.report.output.as_deref())?;
    emit_history_events(
        report,
        config,
        &EmitOpts {
            format,
            stdout: args.report.output.is_none(),
        },
        sink.as_mut(),
    );
    Ok(sink)
}

/// Known (adopted) finding IDs for gate mode. No baseline → every violation
/// gates (fail-closed preserved). A corrupt baseline errors out — it must
/// not silently degrade to "no baseline".
pub(super) fn load_gate_baseline(
    config: &Config,
    current_ids: &[String],
) -> Result<(HashSet<String>, bool)> {
    if !matches!(config.pretender.mode, Mode::Gate) {
        return Ok((HashSet::new(), false));
    }
    match history::load_baseline(Path::new(BASELINE_PATH)) {
        Ok(Some(baseline)) => {
            let known = known_ids(current_ids, &baseline.finding_ids);
            Ok((known, true))
        }
        Ok(None) => Ok((HashSet::new(), false)),
        Err(err) => Err(err.context("gate baseline unreadable — refusing to gate")),
    }
}

/// The current scan's IDs that are also present in the adopted baseline.
fn known_ids(current_ids: &[String], adopted: &[String]) -> HashSet<String> {
    let adopted: HashSet<&String> = adopted.iter().collect();
    current_ids
        .iter()
        .filter(|id| adopted.contains(id))
        .cloned()
        .collect()
}
