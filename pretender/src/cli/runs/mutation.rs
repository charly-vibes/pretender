//! Runner for the `mutation` subcommand: tool invocation and reporting.
//!
//! # Responsibilities
//! `MutationArgs::run`, the report emitter (JSON envelope or human
//! summary), the dry-run planner printer, the human report printer, and
//! the per-mutant survivor/planned-mutant printers.
//!
//! # Rationale
//! Split out of `cli/runs.rs` (pretender-5zo.15.14) along the mutation
//! seam so every file stays under the file-size gate. Bodies moved
//! verbatim except where a function had to split further (the per-mutant
//! loops became `print_planned_mutants`/`print_survivors`) with output
//! unchanged.
use crate::check::collect_input_files;
use crate::cli::args::{MutationArgs, ReportFormat};
use crate::cli::infra::load_config;
use crate::cli::Executable;
use anyhow::{anyhow, Result};
use genesis::envelope::{Envelope, EnvelopeKind};
use pretender::mutation;
use std::path::PathBuf;
use std::process::ExitCode;

impl Executable for MutationArgs {
    fn run(&self) -> Result<ExitCode> {
        let config = load_config()?;
        let files = collect_input_files(&self.paths, &config)?;

        let lang = mutation::primary_lang(&files)
            .ok_or_else(|| anyhow!("no supported source files found in provided paths"))?;

        if self.dry_run {
            return run_mutation_dry_run(&lang, &files);
        }

        let report = mutation::run_mutation(&lang, &files)?;
        emit_mutation_result(&report, self.format)?;

        if report.passes_threshold(self.score_min) {
            Ok(ExitCode::SUCCESS)
        } else {
            Ok(ExitCode::FAILURE)
        }
    }
}

/// Emit the mutation report: a JSON envelope for `--format json`, the
/// human summary otherwise.
fn emit_mutation_result(report: &mutation::MutationReport, format: ReportFormat) -> Result<()> {
    match format {
        ReportFormat::Json => {
            let env = Envelope::success(
                env!("CARGO_PKG_VERSION"),
                env!("CARGO_PKG_VERSION"),
                EnvelopeKind::Check,
                report,
                vec![],
                vec![],
            );
            println!("{}", serde_json::to_string_pretty(&env)?);
        }
        _ => print_mutation_report(report),
    }
    Ok(())
}

fn run_mutation_dry_run(lang: &mutation::MutationLang, paths: &[PathBuf]) -> Result<ExitCode> {
    println!(
        "Dry run: would use {} on {} file(s)",
        lang.tool_name(),
        paths.len()
    );
    let mutants = mutation::list_mutants(lang, paths)?;
    if mutants.is_empty() {
        println!("No mutation sites found.");
        return Ok(ExitCode::SUCCESS);
    }
    println!("Planned mutants ({}):", mutants.len());
    print_planned_mutants(&mutants);
    Ok(ExitCode::SUCCESS)
}

/// Print each planned mutant with its `file:line` (when known) and description.
fn print_planned_mutants(mutants: &[mutation::PlannedMutant]) {
    for m in mutants {
        if m.line > 0 {
            println!("  {}:{}: {}", m.file, m.line, m.description);
        } else {
            println!("  {}: {}", m.file, m.description);
        }
    }
}

fn print_mutation_report(report: &mutation::MutationReport) {
    let pass = if report.score_pct >= 60.0 {
        "✓"
    } else {
        "✗"
    };
    println!(
        "Mutation score: {:.1}% ({}/{} killed) {}",
        report.score_pct, report.killed, report.total, pass
    );
    println!("Tool: {}", report.tool);
    if report.survivors.is_empty() {
        println!("\nAll mutants killed — perfect score!");
        return;
    }
    println!("\nSurviving mutants ({}):", report.survived);
    print_survivors(&report.survivors);
}

/// Print each surviving mutant with its location, description, and the
/// tests that missed it.
fn print_survivors(survivors: &[mutation::SurvivingMutant]) {
    for m in survivors {
        if m.line > 0 {
            print!("  {}:{}", m.file, m.line);
        } else {
            print!("  {}", m.file);
        }
        println!("  {}", m.description);
        for test in &m.missed_tests {
            println!("    missed by: {test}");
        }
    }
}
