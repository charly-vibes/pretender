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
//! findings lives in `event`, store I/O in `store`, human output in
//! `render`.
//!
//! # Rationale
//! Extracted from main.rs (pretender-5zo.2.4); the oversized emitter was
//! split into verbatim-behaving stage helpers while moving so store reads,
//! write-failure messages, and output bytes stay identical, with every
//! function within the structural limits.

mod event;
mod render;
mod store;

use crate::cli::args::ReportFormat;
use crate::report::CheckReport;
use pretender::config;
use pretender::history;
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
    let store = history::EventStore::new(Path::new(".pretender"));
    let Some((new_events, delta)) = store::run_snapshot(report, config, &store) else {
        return;
    };
    if new_events.is_empty() {
        render::clean_run_resolution(opts, sink, &delta);
        return;
    }
    let Some(summary) = store::append_and_summarize(&store, &new_events, &delta) else {
        return;
    };
    render::resolution_delta_if_human(opts, sink, &delta);
    attach_history_and_hints(report, opts, sink, &summary);
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
            let color = opts.stdout && crate::cli::infra::color_enabled();
            let _ = render::recurrence_hints(sink, summary, color);
        }
    }
}
