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

use super::{CheckReport, FileReport, UnitReport};
use super::summary::{fmt_metric, summarize_file, Severity};
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

fn human_palette(color: bool, blocking: bool) -> HumanPalette {
    let red = if color { "\u{1b}[31m" } else { "" };
    let yellow = if color { "\u{1b}[33m" } else { "" };
    let green = if color { "\u{1b}[32m" } else { "" };
    let reset = if color { "\u{1b}[0m" } else { "" };
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

fn write_human_file(
    sink: &mut dyn Write,
    file: &FileReport,
    opts: &HumanOpts<'_>,
    palette: &HumanPalette,
) -> Result<()> {
    let blocking = matches!(opts.mode, Mode::Gate);
    let summary = summarize_file(file, opts.bands);
    let (icon, accent) = match summary.severity {
        Severity::Green => ("✓", palette.green),
        Severity::Yellow => ("⚠", palette.yellow),
        Severity::Red => (palette.icon, if blocking { palette.red } else { palette.yellow }),
    };

    writeln!(
        sink,
        "{}{}{} {} — {}",
        accent, icon, palette.reset, file.path, summary.message
    )?;
    writeln!(sink, "  role: {}", file.role)?;

    write_human_file_violations(sink, file, palette)?;
    write_human_units(sink, file, opts, palette)?;
    write_human_annotations(sink, file, palette)?;
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
            palette.red,
            palette.reset,
            finding.source,
            code,
            finding.line,
            finding.message
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
        writeln!(
            sink,
            "{}{}{} {} test duration violation(s)",
            palette.accent, palette.label, palette.reset, report.test_findings.len(),
        )?;
    }
    Ok(())
}

fn write_human_test_finding(
    sink: &mut dyn Write,
    finding: &TestDurationFinding,
    palette: &HumanPalette,
) -> Result<()> {
    let file_str = finding
        .file
        .as_ref()
        .map(|f| f.display().to_string())
        .unwrap_or_else(|| "?".to_string());
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
