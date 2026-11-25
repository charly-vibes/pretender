//! Runner for the `complexity` subcommand: per-file cyclomatic table.
//!
//! # Responsibilities
//! `ComplexityArgs::run`, the per-file parse-and-print stage
//! (`run_complexity_file`), parse-diagnostic printing, and the sorted
//! per-unit table printer.
//!
//! # Rationale
//! Split out of `cli/runs.rs` (pretender-5zo.15.14) along the complexity
//! seam so every file stays under the file-size gate. Bodies moved
//! verbatim except where a function had to split further (`run`'s loop
//! body became `run_complexity_file`) with output unchanged.
use crate::check::collect_input_files;
use crate::check::scan::get_parser;
use crate::cli::args::ComplexityArgs;
use crate::cli::infra::load_config;
use crate::cli::Executable;
use anyhow::{Context, Result};
use pretender::model::Metric;
use pretender::{metrics, model};
use std::cmp::Reverse;
use std::path::Path;
use std::process::ExitCode;

impl Executable for ComplexityArgs {
    fn run(&self) -> Result<ExitCode> {
        let config = load_config().unwrap_or_default();
        let threshold = config.thresholds.app.cyclomatic_max;
        let files = collect_input_files(&self.paths, &config)?;
        let multi = files.len() > 1;

        for path in &files {
            run_complexity_file(path, threshold, multi)?;
        }
        Ok(ExitCode::SUCCESS)
    }
}

/// Parse one source file and print its diagnostics and complexity table.
fn run_complexity_file(path: &Path, threshold: u32, multi: bool) -> Result<()> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read source file: {}", path.display()))?;
    let parser = get_parser(path)?;
    let (module, diagnostics) = parser.parse(path, &source)?;

    if !diagnostics.is_empty() {
        print_diagnostics(&diagnostics);
    }

    if multi {
        println!("{}:", path.display());
    }

    print_complexity_table(&module, threshold, multi);
    Ok(())
}

/// Print parse diagnostics to stderr with span line ranges, then a blank
/// line separating them from the report body.
fn print_diagnostics(diagnostics: &[model::Diagnostic]) {
    for diag in diagnostics {
        eprintln!("{:?}: {}", diag.severity, diag.message);
        if let Some(span) = &diag.span {
            eprintln!("  at lines {}-{}", span.start_line, span.end_line);
        }
    }
    eprintln!();
}

/// Sort units by cyclomatic score descending and print the per-unit table.
fn print_complexity_table(module: &model::Module, threshold: u32, multi: bool) {
    let metric = metrics::CyclomaticComplexity;
    let mut results: Vec<(String, u32)> = module
        .units
        .iter()
        .map(|u| (u.name.clone(), metric.calculate(u)))
        .collect();

    let indent = if multi { "  " } else { "" };
    results.sort_by_key(|(_, score)| Reverse(*score));
    for (name, score) in &results {
        if *score > threshold {
            println!("{indent}{name}: {score} (threshold: {threshold}) ✗");
        } else {
            println!("{indent}{name}: {score}");
        }
    }
}
