//! Markdown check-report writer.
//!
//! Renders `CheckReport` as a Markdown document with one heading per file,
//! its summary, role, violations, per-unit metrics, and diagnostics.
//!
//! # Responsibilities
//! Emit the markdown format byte-identically to the pre-extraction
//! implementation.
//!
//! # Rationale
//! Split out of the extracted report writers (epic pretender-5zo) into
//! per-file stage helpers so every function fits the gate's limits; bodies
//! moved verbatim from `main.rs`.

use super::summary::{summarize_file, Severity};
use super::{CheckReport, FileReport, UnitReport};
use anyhow::Result;
use pretender::config::Bands;
use std::io::Write;

pub(crate) fn write_markdown_report(
    sink: &mut dyn Write,
    report: &CheckReport,
    bands: &Bands,
) -> Result<()> {
    writeln!(sink, "# Pretender report")?;
    writeln!(sink)?;

    for file in &report.files {
        write_markdown_file(sink, file, bands)?;
    }

    Ok(())
}

fn write_markdown_file(sink: &mut dyn Write, file: &FileReport, bands: &Bands) -> Result<()> {
    let summary = summarize_file(file, bands);
    let icon = match summary.severity {
        Severity::Green => "✓",
        Severity::Yellow => "⚠",
        Severity::Red => "✗",
    };
    writeln!(sink, "## `{}` {} {}", file.path, icon, summary.message)?;
    writeln!(sink)?;
    writeln!(sink, "- Role: `{}`", file.role)?;
    write_markdown_file_violations(sink, file)?;
    write_markdown_units(sink, file)?;
    write_markdown_diagnostics(sink, file)?;
    writeln!(sink)?;
    Ok(())
}

/// A file's file-level violation bullets.
fn write_markdown_file_violations(sink: &mut dyn Write, file: &FileReport) -> Result<()> {
    for violation in &file.file_violations {
        writeln!(
            sink,
            "- File violation: `{}` actual `{:.0}` > limit `{:.0}`",
            violation.metric, violation.actual, violation.limit
        )?;
    }
    Ok(())
}

fn write_markdown_units(sink: &mut dyn Write, file: &FileReport) -> Result<()> {
    for unit in &file.units {
        writeln!(
            sink,
            "- `{}`: cyclomatic `{}`, cognitive `{}`, assertions `{}`, function_lines `{}`, params `{}`, nesting_max `{}`, abc `{:.2}`",
            unit.name,
            unit.metrics.cyclomatic,
            unit.metrics.cognitive,
            unit.metrics.assertions,
            unit.metrics.function_lines,
            unit.metrics.params,
            unit.metrics.nesting_max,
            unit.metrics.abc,
        )?;
        write_markdown_unit_violations(sink, unit)?;
    }
    Ok(())
}

/// One unit's violation bullets, indented under its metrics bullet.
fn write_markdown_unit_violations(sink: &mut dyn Write, unit: &UnitReport) -> Result<()> {
    for violation in &unit.violations {
        writeln!(
            sink,
            "  - violation: `{}` actual `{:.0}` > limit `{:.0}`",
            violation.metric, violation.actual, violation.limit
        )?;
    }
    Ok(())
}

fn write_markdown_diagnostics(sink: &mut dyn Write, file: &FileReport) -> Result<()> {
    if file.diagnostics.is_empty() {
        return Ok(());
    }
    writeln!(sink)?;
    writeln!(sink, "### Diagnostics")?;
    for diagnostic in &file.diagnostics {
        writeln!(sink, "- {}: {}", diagnostic.severity, diagnostic.message)?;
    }
    Ok(())
}
