//! Check-run history (wave-4 ratchet, pretender-5zo.15.4): the violation
//! event log, per-run summaries, and the finding-resolution/baseline seam,
//! split by responsibility:
//! - `events`:  ViolationEvent shape + EventStore append/prune/snapshot I/O
//! - `summary`: hotspot/pattern scoring over retained events (+ area labels)
//! - `delta`:   finding IDs, resolution deltas, snapshots, gate baseline
//! - `clock`:   unix/ISO-8601 time helpers and run IDs
//!
//! The public surface consumed by `crate::events` and `crate::check` is
//! re-exported here unchanged.

mod clock;
mod delta;
mod events;
mod summary;

pub use clock::{make_run_id, now_iso8601, now_unix_secs, unix_to_iso8601};
pub use delta::{
    compute_delta, finding_id, load_baseline, persist_baseline, Baseline, FindingDelta,
    FindingsSnapshot,
};
pub use events::{EventFinding, EventMeasure, EventOrigin, EventStore, ViolationEvent};
pub use summary::{
    compute_summary, HistorySummary, HotspotSummary, PatternSummary, ResolutionStats,
};
