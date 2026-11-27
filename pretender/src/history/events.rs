//! Purpose: the append-only violation event log and per-run finding-ID
//! snapshot storage.
//! Responsibilities: the ViolationEvent wire shape (origin + finding +
//! measure, flattened to the original JSONL layout) and EventStore
//! load/append/prune/write plus snapshot and summary persistence.
//! Rationale: split out of src/history.rs (wave-4 ratchet,
//! pretender-5zo.15.4); append/prune/write decomposed into stage helpers to
//! meet per-function thresholds, semantics unchanged.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use super::clock::{now_unix_secs, parse_timestamp_secs};
use super::delta::FindingsSnapshot;
use super::summary::HistorySummary;

const RETENTION_DAYS: u64 = 90;
const MAX_BYTES: u64 = 10 * 1024 * 1024;

/// Stable identity of the check run that produced the event, plus the event
/// schema version and wall-clock timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventOrigin {
    pub schema_version: u32,
    pub timestamp: String,
    pub run_id: String,
    pub mode: String,
}

/// The violation itself: where, which unit/role, which rule, and how bad.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFinding {
    pub path: String,
    pub unit_name: Option<String>,
    pub role: String,
    pub rule_key: String,
    pub metric_family: String,
    pub scope: String,
    pub severity: String,
}

/// Magnitude vs. threshold, the over-limit delta, and the stable fingerprint.
/// Grouped last so `#[serde(flatten)]` preserves the original JSONL key
/// order (schema_version, timestamp, run_id, mode, path, ..., delta,
/// fingerprint).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMeasure {
    pub actual: f64,
    pub limit: f64,
    pub delta: f64,
    pub fingerprint: String,
}

/// One violation occurrence appended to `history/events.jsonl`. The three
/// substructs are flattened, so the persisted JSONL layout is unchanged
/// (excessive_fields ratchet: no struct exceeds the field cap). When building
/// from a borrowed finding, order the literal so borrows (fingerprint, scope,
/// metric_family) are evaluated before fields are moved out — see
/// `violation_event` in src/events.rs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViolationEvent {
    #[serde(flatten)]
    pub origin: EventOrigin,
    #[serde(flatten)]
    pub finding: EventFinding,
    #[serde(flatten)]
    pub measure: EventMeasure,
}

pub struct EventStore {
    events_path: PathBuf,
    summaries_path: PathBuf,
    snapshot_path: PathBuf,
}

impl EventStore {
    pub fn new(base_dir: &Path) -> Self {
        Self {
            events_path: base_dir.join("history/events.jsonl"),
            summaries_path: base_dir.join("history/summaries.json"),
            snapshot_path: base_dir.join("history/last-findings.json"),
        }
    }

    /// Load the finding-ID snapshot from the most recent check run.
    /// None when no snapshot exists yet (first tracked run).
    pub fn load_last_snapshot(&self) -> Result<Option<FindingsSnapshot>> {
        match fs::read(&self.snapshot_path) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes).with_context(|| {
                format!("failed to parse snapshot: {}", self.snapshot_path.display())
            })?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(anyhow::anyhow!(e).context(format!(
                "failed to read snapshot: {}",
                self.snapshot_path.display()
            ))),
        }
    }

    /// Persist the snapshot of the current run's finding IDs. Called on every
    /// check run, including clean runs.
    pub fn persist_last_snapshot(&self, snap: &FindingsSnapshot) -> Result<()> {
        ensure_parent_dir(&self.snapshot_path)?;
        genesis::fs::atomic_write_json(&self.snapshot_path, snap)
            .with_context(|| format!("failed to write snapshot: {}", self.snapshot_path.display()))
    }

    /// Loads existing events, appends new_events, prunes by age
    /// (RETENTION_DAYS) and by size (MAX_BYTES keeping the newest half),
    /// writes atomically, and returns the retained set.
    ///
    /// Not safe for concurrent callers sharing the same working directory —
    /// last writer wins. Acceptable for V1 pre-commit / CI best-effort use.
    pub fn append_and_prune(&self, new_events: &[ViolationEvent]) -> Result<Vec<ViolationEvent>> {
        ensure_parent_dir(&self.events_path)?;

        let mut events = self.load_events()?;
        events.extend_from_slice(new_events);

        // Age-based retention: drop events with unparseable timestamps (treat as expired).
        let cutoff = now_unix_secs().saturating_sub(RETENTION_DAYS * 86400);
        events.retain(|e| recent_enough(&e.origin.timestamp, cutoff));

        // Keep oldest-first so size-cap trim always discards the oldest half.
        events.sort_by(|a, b| a.origin.timestamp.cmp(&b.origin.timestamp));

        let events = cap_by_size(events);
        self.write_events(&events)?;
        Ok(events)
    }

    fn load_events(&self) -> Result<Vec<ViolationEvent>> {
        if !self.events_path.exists() {
            return Ok(Vec::new());
        }
        let file = fs::File::open(&self.events_path)
            .with_context(|| format!("failed to open events: {}", self.events_path.display()))?;
        let mut events = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            if let Ok(event) = serde_json::from_str::<ViolationEvent>(&line) {
                events.push(event);
            }
        }
        Ok(events)
    }

    // Atomic write via genesis::fs::atomic_write (stage → fsync → rename), so
    // a mid-write kill cannot corrupt the event log.
    fn write_events(&self, events: &[ViolationEvent]) -> Result<()> {
        let mut bytes = Vec::new();
        for event in events {
            writeln!(bytes, "{}", serde_json::to_string(event)?)?;
        }
        genesis::fs::atomic_write(&self.events_path, &bytes)
            .with_context(|| format!("failed to finalize events: {}", self.events_path.display()))
    }

    pub fn persist_summary(&self, summary: &HistorySummary) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(summary)?;
        fs::write(&self.summaries_path, bytes).with_context(|| {
            format!(
                "failed to write summaries: {}",
                self.summaries_path.display()
            )
        })
    }
}

// Create the parent directory of an event/snapshot file unless it is empty.
fn ensure_parent_dir(path: &Path) -> Result<()> {
    let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) else {
        return Ok(());
    };
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create history dir: {}", parent.display()))
}

// Age-based retention: drop events with unparseable timestamps (treat as expired).
fn recent_enough(timestamp: &str, cutoff: u64) -> bool {
    parse_timestamp_secs(timestamp).is_some_and(|secs| secs >= cutoff)
}

// Size cap: estimate bytes in memory to avoid a double disk write; once over
// MAX_BYTES keep only the newest half (events are sorted oldest-first).
fn cap_by_size(events: Vec<ViolationEvent>) -> Vec<ViolationEvent> {
    let byte_estimate: usize = events
        .iter()
        .map(|e| serde_json::to_string(e).map(|s| s.len() + 1).unwrap_or(0))
        .sum();
    if byte_estimate as u64 > MAX_BYTES {
        let keep_from = events.len() / 2;
        return events[keep_from..].to_vec();
    }
    events
}
