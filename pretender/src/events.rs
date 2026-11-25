//! History/event emission.
//!
//! Turns a completed check report into history events: computes the
//! finding delta against the previous snapshot, persists snapshots and
//! summaries, appends events, and renders the human-facing resolution and
//! recurrence output.
//!
//! # Responsibilities
//! `emit_history_events` is the single entry point the check pipeline
//! calls; it orchestrates snapshot persistence (every run, clean or not),
//! delta computation, event append + summary rebuild, and the human-only
//! delta line / recurrence hints. Event construction from violation
//! findings lives in `all_violation_events`.
//!
//! # Rationale
//! Extracted from main.rs (pretender-5zo.2.4); the oversized emitter was
//! split into verbatim-behaving stage helpers while moving so store reads,
//! write-failure messages, and output bytes stay identical, with every
//! function within the structural limits.
use crate::check::gate::{violation_finding_id, violation_findings};
use crate::cli::args::ReportFormat;
use crate::cli::infra::color_enabled;
use crate::report::CheckReport;
use anyhow::Result;
use pretender::config;
use pretender::history;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

/// Options controlling where and how history output is rendered.
pub(crate) struct EmitOpts {
    pub(crate) format: ReportFormat,
    pub(crate) stdout: bool,
}

pub(crate) fn emit_history_events(
    report: &mut CheckReport,
    config: &config::Config,
    opts: &EmitOpts,
    sink: &mut dyn Write,
) {
    let timestamp = history::now_iso8601();
    let run_id = history::make_run_id();
    let new_events = all_violation_events(report, &run_id, &timestamp, config);
    // Snapshot runs on EVERY check (including clean runs) so that "fixed" is
    // distinguishable from "not rescanned" — the core of ticket u8a.
    let current_ids: Vec<String> = new_events.iter().map(|e| e.fingerprint.clone()).collect();
    let store = history::EventStore::new(Path::new(".pretender"));
    let Some(delta) = load_delta(&store, &run_id, &timestamp, &current_ids) else {
        return;
    };

    if new_events.is_empty() {
        write_clean_run_resolution(opts, sink, &delta);
        return;
    }
    let Some(summary) = append_events_and_summarize(&store, &new_events, &delta) else {
        return;
    };
    if matches!(opts.format, ReportFormat::Human) {
        let color = opts.stdout && color_enabled();
        let _ = write_resolution_delta(sink, &delta, color);
    }
    attach_history_and_hints(report, opts, sink, &summary);
}

/// Load the previous snapshot, compute the finding delta, then persist the
/// fresh snapshot with first_seen carried forward. Returns `None` (after
/// the read-failure message) when the history store is unreadable.
fn load_delta(
    store: &history::EventStore,
    run_id: &str,
    timestamp: &str,
    current_ids: &[String],
) -> Option<history::FindingDelta> {
    let previous = match store.load_last_snapshot() {
        Ok(prev) => prev,
        Err(err) => {
            eprintln!("pretender: history read failed: {err}");
            return None;
        }
    };
    let delta = history::compute_delta(
        previous.as_ref().map(|s| s.finding_ids.as_slice()),
        current_ids,
    );
    let first_seen = carry_first_seen(previous.as_ref(), current_ids, timestamp);
    let snapshot = history::FindingsSnapshot {
        schema_version: 1,
        run_id: run_id.to_string(),
        timestamp: timestamp.to_string(),
        finding_ids: current_ids.to_vec(),
        first_seen,
    };
    persist_current_snapshot(store, &snapshot);
    Some(delta)
}

/// first_seen map for the new snapshot: previous entries kept, unknown
/// fingerprints stamped with this run's timestamp so persistence duration
/// is tracked (consumed later by dj5's gate-findings export).
fn carry_first_seen(
    previous: Option<&history::FindingsSnapshot>,
    current_ids: &[String],
    timestamp: &str,
) -> HashMap<String, String> {
    let mut first_seen = previous.map(|s| s.first_seen.clone()).unwrap_or_default();
    for id in current_ids {
        first_seen
            .entry(id.clone())
            .or_insert_with(|| timestamp.to_string());
    }
    first_seen
}

/// Snapshot write failures are reported but do not abort the run.
fn persist_current_snapshot(store: &history::EventStore, snapshot: &history::FindingsSnapshot) {
    if let Err(err) = store.persist_last_snapshot(snapshot) {
        eprintln!("pretender: history write failed: {err}");
    }
}

/// Human-only resolution line after a run with no violations.
fn write_clean_run_resolution(
    opts: &EmitOpts,
    sink: &mut dyn Write,
    delta: &history::FindingDelta,
) {
    if !delta.fixed.is_empty() && matches!(opts.format, ReportFormat::Human) {
        let color = opts.stdout && color_enabled();
        let (green, reset) = if color {
            ("\u{1b}[32m", "\u{1b}[0m")
        } else {
            ("", "")
        };
        let _ = writeln!(
            sink,
            "{green}✓{reset} resolution: {} fixed since last check",
            delta.fixed.len()
        );
    }
}

/// Append the run's events (pruning history) and rebuild the summary with
/// resolution stats attached; `None` after an append write failure.
fn append_events_and_summarize(
    store: &history::EventStore,
    new_events: &[history::ViolationEvent],
    delta: &history::FindingDelta,
) -> Option<history::HistorySummary> {
    let all_events = match store.append_and_prune(new_events) {
        Ok(events) => events,
        Err(err) => {
            eprintln!("pretender: history write failed: {err}");
            return None;
        }
    };
    let mut summary = history::compute_summary(&all_events);
    summary.resolution = Some(history::ResolutionStats {
        fixed: delta.fixed.len(),
        still_open: delta.still_open.len(),
        new: delta.new.len(),
        rate: delta.resolution_rate,
    });
    let _ = store.persist_summary(&summary);
    Some(summary)
}

/// Attach the summary to the report (JSON output) and print recurrence
/// hints when recurring content exists.
fn attach_history_and_hints(
    report: &mut CheckReport,
    opts: &EmitOpts,
    sink: &mut dyn Write,
    summary: &history::HistorySummary,
) {
    if !summary.top_hotspots.is_empty()
        || !summary.top_patterns.is_empty()
        || summary.resolution.is_some()
    {
        report.history = Some(summary.clone());
        if matches!(opts.format, ReportFormat::Human) {
            let color = opts.stdout && color_enabled();
            let _ = write_recurrence_hints(sink, summary, color);
        }
    }
}

/// Human-readable delta line: "resolution: 3 fixed, 5 still open, 2 new (43%)"
fn write_resolution_delta(
    sink: &mut dyn Write,
    delta: &history::FindingDelta,
    color: bool,
) -> Result<()> {
    let (cyan, reset) = if color {
        ("\u{1b}[36m", "\u{1b}[0m")
    } else {
        ("", "")
    };
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

fn metric_to_family(metric: &str) -> &str {
    match metric {
        "cognitive" | "cyclomatic" | "nesting" | "abc" => "complexity",
        "params" | "function_lines" => "size",
        "duplication_pct" => "duplication",
        "min_assertions" => "testing",
        other => other,
    }
}

fn all_violation_events(
    report: &CheckReport,
    run_id: &str,
    timestamp: &str,
    config: &config::Config,
) -> Vec<history::ViolationEvent> {
    let mode = format!("{:?}", config.pretender.mode).to_ascii_lowercase();
    violation_findings(report)
        .into_iter()
        .map(|finding| violation_event(finding, run_id, timestamp, &mode))
        .collect()
}

/// Unit-scoped when the finding carries a unit name, file-scoped otherwise.
fn finding_scope(unit_name: &Option<String>) -> &'static str {
    if unit_name.is_some() {
        "unit"
    } else {
        "file"
    }
}

/// Build the history event for one violation finding.
fn violation_event(
    finding: crate::check::gate::ViolationFinding,
    run_id: &str,
    timestamp: &str,
    mode: &str,
) -> history::ViolationEvent {
    let metric_family = metric_to_family(&finding.metric).to_string();
    let scope = finding_scope(&finding.unit_name).to_string();
    let fingerprint = violation_finding_id(&finding);
    history::ViolationEvent {
        schema_version: 1,
        timestamp: timestamp.to_string(),
        run_id: run_id.to_string(),
        mode: mode.to_string(),
        path: finding.path,
        unit_name: finding.unit_name,
        role: finding.role,
        rule_key: finding.rule_key,
        metric_family,
        scope,
        severity: "red".to_string(),
        actual: finding.actual,
        limit: finding.limit,
        delta: finding.actual - finding.limit,
        fingerprint,
    }
}

fn write_recurrence_hints(
    sink: &mut dyn Write,
    summary: &history::HistorySummary,
    color: bool,
) -> Result<()> {
    let cyan = if color { "\u{1b}[36m" } else { "" };
    let bold = if color { "\u{1b}[1m" } else { "" };
    let reset = if color { "\u{1b}[0m" } else { "" };

    writeln!(sink)?;
    writeln!(sink, "{bold}Recurring violations{reset}")?;
    for h in &summary.top_hotspots {
        writeln!(
            sink,
            "  {cyan}hotspot{reset} {} — {} occurrences across {} days",
            h.fingerprint, h.count, h.distinct_days
        )?;
    }
    for p in &summary.top_patterns {
        writeln!(
            sink,
            "  {cyan}pattern{reset} {} in {} ({}) — {} occurrences across {} files",
            p.rule_key, p.area, p.role, p.count, p.distinct_files
        )?;
    }
    Ok(())
}
