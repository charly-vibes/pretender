//! Rendering of the doctor report as human text or JSON.

use anyhow::Result;
use genesis::doctor::{CheckStatus, DoctorReport};
use genesis::guide::OutputFormat;
use std::process::ExitCode;

pub(super) fn render(report: &DoctorReport, format: OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => print_json(report)?,
        OutputFormat::Human => print_human(report),
    }
    Ok(())
}

/// Failure exit when any check ended in a failure state.
pub(super) fn exit_code(report: &DoctorReport) -> ExitCode {
    if report.summary.has_failures() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn print_human(report: &DoctorReport) {
    for check in &report.checks {
        let prefix = match check.status {
            CheckStatus::Pass => "✓",
            CheckStatus::Warn => "⚠",
            CheckStatus::Fail => "✗",
        };
        println!("{prefix} {} — {}", check.name, check.message);
    }
    let total = report.checks.len();
    let passed = report.summary.pass;
    println!("\n{passed}/{total} checks passed");
}

fn print_json(report: &DoctorReport) -> Result<()> {
    let envelope = report.to_envelope(env!("CARGO_PKG_VERSION"));
    let json = serde_json::to_string_pretty(&envelope)?;
    println!("{json}");
    Ok(())
}
