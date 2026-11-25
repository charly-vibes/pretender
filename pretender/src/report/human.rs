//! Human-readable check-report writer.
//!
//! Renders `CheckReport` to the sink with ANSI accents when color is on,
//! gate vs. advisory icons/labels, per-file summaries, per-unit metric
//! lines, diagnostics/external findings, and the test-duration section.
//!
//! # Responsibilities
//! Emit the human format byte-identically to the pre-extraction
//! implementation; own `HumanOpts` and the per-pass color palette.
//!
//! # Rationale
//! Split out of the extracted report writers (epic pretender-5zo) into
//! stage helpers so every function fits the gate's limits; bodies moved
//! verbatim from `main.rs`.

use super::summary::{fmt_metric, summarize_file, FileSummary, Severity};
use super::{CheckReport, FileReport, UnitReport};
use anyhow::Result;
use pretender::config::{Bands, Mode};
use pretender::test_report::TestDurationFinding;
use std::io::Write;

/// Presentation options for the human-readable check report.
pub(crate) struct HumanOpts<'a> {
    pub(crate) color: bool,
    pub(crate) bands: &'a Bands,
    pub(crate) mode: Mode,
    pub(crate) verbose: bool,
}

/// Color codes and icons shared by one human-report rendering pass.
struct HumanPalette {
    red: &'static str,
    yellow: &'static str,
    green: &'static str,
    reset: &'static str,
    icon: &'static str,
    label: &'static str,
    accent: &'static str,
}

/// ANSI code when color is on, empty string otherwise.
fn color_or_empty(color: bool, code: &'static str) -> &'static str {
    if color {
        code
    } else {
        ""
    }
}

fn human_palette(color: bool, blocking: bool) -> HumanPalette {
    let red = color_or_empty(color, "\u{1b}[31m");
    let yellow = color_or_empty(color, "\u{1b}[33m");
    let green = color_or_empty(color, "\u{1b}[32m");
    let reset = color_or_empty(color, "\u{1b}[0m");
    let (icon, label, accent) = if blocking {
        ("✗", "VIOLATION", red)
    } else {
        ("⚠", "ADVISORY", yellow)
    };
    HumanPalette {
        red,
        yellow,
        green,
        reset,
        icon,
        label,
        accent,
    }
}

pub(crate) fn write_human_report(
    sink: &mut dyn Write,
    report: &CheckReport,
    opts: &HumanOpts<'_>,
) -> Result<()> {
    let blocking = matches!(opts.mode, Mode::Gate);
    let palette = human_palette(opts.color, blocking);
    for file in &report.files {
        write_human_file(sink, file, opts, &palette)?;
    }
    write_human_test_findings(sink, report, blocking, &palette)
}

/// The (icon, accent, reset) triple a file's severity renders with. Gate
/// mode stains red findings with the hard-fail icon; advisory modes keep
/// them yellow.
fn severity_marker(
    severity: Severity,
    palette: &HumanPalette,
    blocking: bool,
) -> (&'static str, &'static str, &'static str) {
    let red_accent = if blocking {
        palette.red
    } else {
        palette.yellow
    };
    match severity {
        Severity::Green => ("✓", palette.green, palette.reset),
        Severity::Yellow => ("⚠", palette.yellow, palette.reset),
        Severity::Red => (palette.icon, red_accent, palette.reset),
    }
}

fn write_human_file(
    sink: &mut dyn Write,
    file: &FileReport,
    opts: &HumanOpts<'_>,
    palette: &HumanPalette,
) -> Result<()> {
    let blocking = matches!(opts.mode, Mode::Gate);
    let summary = summarize_file(file, opts.bands);
    let marker = severity_marker(summary.severity, palette, blocking);
    write_human_file_heading(sink, file, &summary, marker)?;
    writeln!(sink, "  role: {}", file.role)?;
    write_human_file_violations(sink, file, palette)?;
    write_human_units(sink, file, opts, palette)?;
    write_human_annotations(sink, file, palette)?;
    Ok(())
}

/// The heading line introducing one file's section.
fn write_human_file_heading(
    sink: &mut dyn Write,
    file: &FileReport,
    summary: &FileSummary,
    marker: (&'static str, &'static str, &'static str),
) -> Result<()> {
    let (icon, accent, reset) = marker;
    writeln!(
        sink,
        "{}{}{} {} — {}",
        accent, icon, reset, file.path, summary.message
    )?;
    Ok(())
}

fn write_human_file_violations(
    sink: &mut dyn Write,
    file: &FileReport,
    palette: &HumanPalette,
) -> Result<()> {
    for violation in &file.file_violations {
        writeln!(
            sink,
            "  {}{}{} {} {} > {}",
            palette.accent,
            palette.label,
            palette.reset,
            violation.metric,
            fmt_metric(violation.actual),
            fmt_metric(violation.limit),
        )?;
    }
    Ok(())
}

fn write_human_units(
    sink: &mut dyn Write,
    file: &FileReport,
    opts: &HumanOpts<'_>,
    palette: &HumanPalette,
) -> Result<()> {
    for unit in &file.units {
        write_human_unit(sink, unit, opts, palette)?;
    }
    Ok(())
}

fn write_human_unit(
    sink: &mut dyn Write,
    unit: &UnitReport,
    opts: &HumanOpts<'_>,
    palette: &HumanPalette,
) -> Result<()> {
    let has_violations = !unit.violations.is_empty();
    if !(opts.verbose || has_violations) {
        return Ok(());
    }
    writeln!(sink, "{}", unit_metrics_line(unit))?;
    write_human_unit_violations(sink, unit, palette)?;
    Ok(())
}

/// A unit's violation lines, indented under its metrics line.
fn write_human_unit_violations(
    sink: &mut dyn Write,
    unit: &UnitReport,
    palette: &HumanPalette,
) -> Result<()> {
    for violation in &unit.violations {
        writeln!(
            sink,
            "    {}{}{} {} {} > {}",
            palette.accent,
            palette.label,
            palette.reset,
            violation.metric,
            fmt_metric(violation.actual),
            fmt_metric(violation.limit),
        )?;
    }
    Ok(())
}

/// Metric summary line for one unit, exactly as the human report prints it.
fn unit_metrics_line(unit: &UnitReport) -> String {
    format!(
        "  {}: cyclomatic={}, cognitive={}, assertions={}, function_lines={}, params={}, nesting_max={}, abc={:.2}",
        unit.name,
        unit.metrics.cyclomatic,
        unit.metrics.cognitive,
        unit.metrics.assertions,
        unit.metrics.function_lines,
        unit.metrics.params,
        unit.metrics.nesting_max,
        unit.metrics.abc,
    )
}

fn write_human_annotations(
    sink: &mut dyn Write,
    file: &FileReport,
    palette: &HumanPalette,
) -> Result<()> {
    for diagnostic in &file.diagnostics {
        eprintln!("  {:?}: {}", diagnostic.severity, diagnostic.message);
    }
    for finding in &file.external_findings {
        let code = finding.code.as_deref().unwrap_or("-");
        writeln!(
            sink,
            "  {}EXTERNAL{} {} {} line {}: {}",
            palette.red, palette.reset, finding.source, code, finding.line, finding.message
        )?;
    }
    Ok(())
}

fn write_human_test_findings(
    sink: &mut dyn Write,
    report: &CheckReport,
    blocking: bool,
    palette: &HumanPalette,
) -> Result<()> {
    if report.test_findings.is_empty() {
        return Ok(());
    }
    writeln!(sink, "\nTest duration findings:")?;
    for finding in &report.test_findings {
        write_human_test_finding(sink, finding, palette)?;
    }
    if blocking {
        write_human_test_summary(sink, &report.test_findings, palette)?;
    }
    Ok(())
}

/// The gate-mode count line closing the test-duration section.
fn write_human_test_summary(
    sink: &mut dyn Write,
    findings: &[TestDurationFinding],
    palette: &HumanPalette,
) -> Result<()> {
    writeln!(
        sink,
        "{}{}{} {} test duration violation(s)",
        palette.accent,
        palette.label,
        palette.reset,
        findings.len(),
    )?;
    Ok(())
}

/// Path label for a duration finding; `?` when the file is unknown.
fn finding_file_label(finding: &TestDurationFinding) -> String {
    finding
        .file
        .as_ref()
        .map(|f| f.display().to_string())
        .unwrap_or_else(|| "?".to_string())
}

fn write_human_test_finding(
    sink: &mut dyn Write,
    finding: &TestDurationFinding,
    palette: &HumanPalette,
) -> Result<()> {
    let file_str = finding_file_label(finding);
    writeln!(
        sink,
        "  {}{}{} {} {} {}: {}ms > {}ms",
        palette.red,
        palette.icon,
        palette.reset,
        finding.role,
        finding.test_name,
        file_str,
        finding.observed_ms,
        finding.threshold_ms,
    )?;
    Ok(())
}
