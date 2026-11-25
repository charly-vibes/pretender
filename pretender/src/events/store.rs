//! Snapshot persistence, delta computation, and event append + summary
//! rebuild against the history store.

use crate::report::CheckReport;
use pretender::config;
use pretender::history;
use std::collections::HashMap;

/// Snapshot + delta for one check run: computes this run's violation
/// events, loads the previous snapshot and computes the finding delta,
/// then persists the fresh snapshot. Runs on EVERY check (including clean
/// runs) so that "fixed" is distinguishable from "not rescanned" — the
/// core of ticket u8a. `None` (after the read-failure message) when the
/// history store is unreadable.
pub(super) fn run_snapshot(
    report: &CheckReport,
    config: &config::Config,
    store: &history::EventStore,
) -> Option<(Vec<history::ViolationEvent>, history::FindingDelta)> {
    let timestamp = history::now_iso8601();
    let run_id = history::make_run_id();
    let new_events = super::event::all_violation_events(report, &run_id, &timestamp, config);
    let current_ids = super::event::finding_ids(&new_events);
    let delta = load_delta(store, &run_id, &timestamp, &current_ids)?;
    Some((new_events, delta))
}

/// Load the previous snapshot, compute the finding delta, then persist the
/// fresh snapshot with first_seen carried forward.
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
    let prev_ids = previous.as_ref().map(|s| s.finding_ids.as_slice());
    let delta = history::compute_delta(prev_ids, current_ids);
    let snapshot = run_snapshot_record(run_id, timestamp, current_ids, previous.as_ref());
    persist_current_snapshot(store, &snapshot);
    Some(delta)
}

/// Fresh snapshot record with first_seen carried forward from `previous`.
fn run_snapshot_record(
    run_id: &str,
    timestamp: &str,
    current_ids: &[String],
    previous: Option<&history::FindingsSnapshot>,
) -> history::FindingsSnapshot {
    history::FindingsSnapshot {
        schema_version: 1,
        run_id: run_id.to_string(),
        timestamp: timestamp.to_string(),
        finding_ids: current_ids.to_vec(),
        first_seen: carry_first_seen(previous, current_ids, timestamp),
    }
}

/// Snapshot write failures are reported but do not abort the run.
fn persist_current_snapshot(store: &history::EventStore, snapshot: &history::FindingsSnapshot) {
    if let Err(err) = store.persist_last_snapshot(snapshot) {
        eprintln!("pretender: history write failed: {err}");
    }
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

/// Append the run's events (pruning history) and rebuild the summary with
/// resolution stats attached; `None` after an append write failure.
pub(super) fn append_and_summarize(
    store: &history::EventStore,
    new_events: &[history::ViolationEvent],
    delta: &history::FindingDelta,
) -> Option<history::HistorySummary> {
    let all_events = append_reported(store, new_events)?;
    let summary = resolution_summary(&all_events, delta);
    let _ = store.persist_summary(&summary);
    Some(summary)
}

/// Event-store append with the standard write-failure report.
fn append_reported(
    store: &history::EventStore,
    new_events: &[history::ViolationEvent],
) -> Option<Vec<history::ViolationEvent>> {
    match store.append_and_prune(new_events) {
        Ok(events) => Some(events),
        Err(err) => {
            eprintln!("pretender: history write failed: {err}");
            None
        }
    }
}

/// Summary over the full history with this run's resolution stats attached.
fn resolution_summary(
    all_events: &[history::ViolationEvent],
    delta: &history::FindingDelta,
) -> history::HistorySummary {
    let mut summary = history::compute_summary(all_events);
    summary.resolution = Some(history::ResolutionStats {
        fixed: delta.fixed.len(),
        still_open: delta.still_open.len(),
        new: delta.new.len(),
        rate: delta.resolution_rate,
    });
    summary
}
