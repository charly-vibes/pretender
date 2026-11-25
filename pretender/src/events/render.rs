//! Human-facing history output: resolution lines and recurrence hints.

use crate::cli::args::ReportFormat;
use crate::cli::infra::color_enabled;
use pretender::history;
use std::io::Write;

use super::EmitOpts;

/// Human-only resolution line after a run with no violations.
pub(super) fn clean_run_resolution(
    opts: &EmitOpts,
    sink: &mut dyn Write,
    delta: &history::FindingDelta,
) {
    if delta.fixed.is_empty() || !matches!(opts.format, ReportFormat::Human) {
        return;
    }
    let (green, reset) = color_pair(opts.stdout && color_enabled(), "\u{1b}[32m");
    let _ = writeln!(
        sink,
        "{green}✓{reset} resolution: {} fixed since last check",
        delta.fixed.len()
    );
}

/// Human-only delta line: "resolution: 3 fixed, 5 still open, 2 new (43%)"
pub(super) fn resolution_delta_if_human(
    opts: &EmitOpts,
    sink: &mut dyn Write,
    delta: &history::FindingDelta,
) {
    if !matches!(opts.format, ReportFormat::Human) {
        return;
    }
    let color = opts.stdout && color_enabled();
    let _ = write_resolution_delta(sink, delta, color);
}

fn write_resolution_delta(
    sink: &mut dyn Write,
    delta: &history::FindingDelta,
    color: bool,
) -> std::io::Result<()> {
    let (cyan, reset) = color_pair(color, "\u{1b}[36m");
    let rate = delta
        .resolution_rate
        .map(|r| format!(" ({:.0}%)", r * 100.0))
        .unwrap_or_default();
    writeln!(
        sink,
        "{cyan}→{reset} resolution: {} fixed, {} still open, {} new{rate}",
        delta.fixed.len(),
        delta.still_open.len(),
        delta.new.len()
    )?;
    Ok(())
}

/// Recurrence hints block printed after runs with recurring content.
pub(super) fn recurrence_hints(
    sink: &mut dyn Write,
    summary: &history::HistorySummary,
    color: bool,
) -> std::io::Result<()> {
    let (cyan, reset) = color_pair(color, "\u{1b}[36m");
    let bold = if color { "\u{1b}[1m" } else { "" };
    writeln!(sink)?;
    writeln!(sink, "{bold}Recurring violations{reset}")?;
    write_hotspots(sink, summary, cyan, reset)?;
    write_patterns(sink, summary, cyan, reset)
}

/// Per-fingerprint hotspot lines.
fn write_hotspots(
    sink: &mut dyn Write,
    summary: &history::HistorySummary,
    cyan: &str,
    reset: &str,
) -> std::io::Result<()> {
    for h in &summary.top_hotspots {
        writeln!(
            sink,
            "  {cyan}hotspot{reset} {} — {} occurrences across {} days",
            h.fingerprint, h.count, h.distinct_days
        )?;
    }
    Ok(())
}

/// Per-rule/area pattern lines.
fn write_patterns(
    sink: &mut dyn Write,
    summary: &history::HistorySummary,
    cyan: &str,
    reset: &str,
) -> std::io::Result<()> {
    for p in &summary.top_patterns {
        writeln!(
            sink,
            "  {cyan}pattern{reset} {} in {} ({}) — {} occurrences across {} files",
            p.rule_key, p.area, p.role, p.count, p.distinct_files
        )?;
    }
    Ok(())
}

/// (escape, reset) pair for `code` when color output is enabled.
fn color_pair(color: bool, code: &'static str) -> (&'static str, &'static str) {
    if color {
        (code, "\u{1b}[0m")
    } else {
        ("", "")
    }
}
