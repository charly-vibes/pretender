//! HTML check-report writer.
//!
//! Renders `CheckReport` as a standalone HTML document with one section per
//! file, severity-colored headings, and escaped user-controlled strings.
//!
//! # Responsibilities
//! Emit the HTML format byte-identically to the pre-extraction
//! implementation; own `html_escape`.
//!
//! # Rationale
//! Split out of the extracted report writers (epic pretender-5zo) into
//! per-file stage helpers so every function fits the gate's limits; bodies
//! moved verbatim from `main.rs`.

use super::{CheckReport, FileReport, UnitReport};
use super::summary::{summarize_file, FileSummary, Severity};
use anyhow::Result;
use pretender::config::Bands;
use std::io::Write;

pub(crate) fn write_html_report(
    sink: &mut dyn Write,
    report: &CheckReport,
    bands: &Bands,
) -> Result<()> {
    writeln!(sink, "<!doctype html>")?;
    writeln!(sink, "<html><head><meta charset=\"utf-8\"><title>Pretender report</title><style>body{{font-family:system-ui,sans-serif;max-width:960px;margin:2rem auto;padding:0 1rem}} .green{{color:#18794e}} .yellow{{color:#9a6700}} .red{{color:#cf222e}} code{{background:#f6f8fa;padding:.1rem .3rem;border-radius:4px}} ul{{line-height:1.5}}</style></head><body>")?;
    writeln!(sink, "<h1>Pretender report</h1>")?;

    for file in &report.files {
        write_html_file(sink, file, bands)?;
    }

    writeln!(sink, "</body></html>")?;
    Ok(())
}

fn write_html_file(sink: &mut dyn Write, file: &FileReport, bands: &Bands) -> Result<()> {
    let summary = summarize_file(file, bands);
    write_html_heading(sink, file, &summary)?;
    write_html_file_violations(sink, file)?;
    write_html_units(sink, file)?;
    writeln!(sink, "</ul>")?;
    write_html_diagnostics(sink, file)?;
    writeln!(sink, "</section>")?;
    Ok(())
}

fn write_html_heading(
    sink: &mut dyn Write,
    file: &FileReport,
    summary: &FileSummary,
) -> Result<()> {
    let (icon, class_name) = match summary.severity {
        Severity::Green => ("✓", "green"),
        Severity::Yellow => ("⚠", "yellow"),
        Severity::Red => ("✗", "red"),
    };
    writeln!(
        sink,
        "<section><h2><code>{}</code> <span class=\"{}\">{} {}</span></h2>",
        html_escape(&file.path),
        class_name,
        icon,
        html_escape(&summary.message)
    )?;
    writeln!(
        sink,
        "<p>Role: <code>{}</code></p><ul>",
        html_escape(&file.role)
    )?;
    Ok(())
}

fn write_html_file_violations(sink: &mut dyn Write, file: &FileReport) -> Result<()> {
    for violation in &file.file_violations {
        writeln!(
            sink,
            "<li>File violation: <code>{}</code> actual <code>{:.0}</code> &gt; limit <code>{:.0}</code></li>",
            html_escape(&violation.metric),
            violation.actual,
            violation.limit
        )?;
    }
    Ok(())
}

fn write_html_units(sink: &mut dyn Write, file: &FileReport) -> Result<()> {
    for unit in &file.units {
        writeln!(
            sink,
            "<li><code>{}</code>: cyclomatic <code>{}</code>, cognitive <code>{}</code>, assertions <code>{}</code>, function_lines <code>{}</code>, params <code>{}</code>, nesting_max <code>{}</code>, abc <code>{:.2}</code>",
            html_escape(&unit.name),
            unit.metrics.cyclomatic,
            unit.metrics.cognitive,
            unit.metrics.assertions,
            unit.metrics.function_lines,
            unit.metrics.params,
            unit.metrics.nesting_max,
            unit.metrics.abc,
        )?;
        write_html_unit_violations(sink, unit)?;
        writeln!(sink, "</li>")?;
    }
    Ok(())
}

fn write_html_unit_violations(sink: &mut dyn Write, unit: &UnitReport) -> Result<()> {
    if unit.violations.is_empty() {
        return Ok(());
    }
    writeln!(sink, "<ul>")?;
    for violation in &unit.violations {
        writeln!(
            sink,
            "<li>violation: <code>{}</code> actual <code>{:.0}</code> &gt; limit <code>{:.0}</code></li>",
            html_escape(&violation.metric),
            violation.actual,
            violation.limit
        )?;
    }
    writeln!(sink, "</ul>")?;
    Ok(())
}

fn write_html_diagnostics(sink: &mut dyn Write, file: &FileReport) -> Result<()> {
    if file.diagnostics.is_empty() {
        return Ok(());
    }
    writeln!(sink, "<h3>Diagnostics</h3><ul>")?;
    for diagnostic in &file.diagnostics {
        writeln!(
            sink,
            "<li>{}: {}</li>",
            html_escape(&diagnostic.severity),
            html_escape(&diagnostic.message)
        )?;
    }
    writeln!(sink, "</ul>")?;
    Ok(())
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
