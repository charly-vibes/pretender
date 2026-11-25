//! Purpose: finding identity, resolution deltas, and the gate baseline.
//! Responsibilities: stable finding IDs, the finding-ID snapshot shape,
//! fixed/still_open/new classification, baseline load/adopt persistence.
//! Rationale: split out of src/history.rs (wave-4 ratchet,
//! pretender-5zo.15.4); bodies moved verbatim.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

use super::clock::now_iso8601;

/// Stable finding identifier: path::unit_name::rule_key. Built from the unit
/// name (not line numbers) so edits that shift lines keep the same ID
/// (ticket u8a acceptance).
pub fn finding_id(path: &str, unit_name: &str, rule_key: &str) -> String {
    format!("{path}::{unit_name}::{rule_key}")
}

/// Delta between the previous check's finding IDs and the current scan's.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FindingDelta {
    pub fixed: Vec<String>,
    pub still_open: Vec<String>,
    pub new: Vec<String>,
    /// fixed / (fixed + still_open); None when there is no previous snapshot
    /// (first tracked run) or nothing was ever flagged.
    pub resolution_rate: Option<f64>,
}

/// Snapshot of the finding IDs observed by the most recent check run.
/// Persisted on EVERY run (including clean runs) so "fixed" is
/// distinguishable from "not rescanned".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingsSnapshot {
    pub schema_version: u32,
    pub run_id: String,
    pub timestamp: String,
    pub finding_ids: Vec<String>,
    /// finding ID -> ISO timestamp of first sighting (for dj5: how long a
    /// finding has persisted).
    pub first_seen: HashMap<String, String>,
}

/// Compare previous finding IDs against the current scan and classify each
/// finding: fixed (seen before, gone now), still_open, new.
/// `resolution_rate` = fixed / (fixed + still_open); None when nothing was
/// previously tracked (first run) or when there was nothing to resolve.
pub fn compute_delta(previous: Option<&[String]>, current: &[String]) -> FindingDelta {
    let prev: HashSet<&str> = previous
        .map(|ids| ids.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();
    let cur: HashSet<&str> = current.iter().map(|s| s.as_str()).collect();
    let fixed: Vec<String> = prev.difference(&cur).map(|s| s.to_string()).collect();
    let still_open: Vec<String> = cur.intersection(&prev).map(|s| s.to_string()).collect();
    let new: Vec<String> = cur.difference(&prev).map(|s| s.to_string()).collect();
    let resolution_rate = resolution_rate(previous.is_none(), &fixed, &still_open);
    FindingDelta {
        fixed,
        still_open,
        new,
        resolution_rate,
    }
}

/// fixed / (fixed + still_open); None when nothing was previously tracked
/// (first run) or when there was nothing to resolve.
fn resolution_rate(no_previous: bool, fixed: &[String], still_open: &[String]) -> Option<f64> {
    if no_previous || (fixed.is_empty() && still_open.is_empty()) {
        return None;
    }
    let tracked = fixed.len() + still_open.len();
    Some(fixed.len() as f64 / (tracked.max(1) as f64))
}

/// Gate-mode baseline (gently-if4): the set of stable finding IDs a team has
/// explicitly adopted. With a baseline present, `check --mode gate` fails only
/// on findings whose stable ID is NOT in the baseline (genuinely new); known
/// violations pass with a summary line. No baseline → gate fails on any
/// violation (fail-closed preserved).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Baseline {
    pub schema_version: u32,
    /// ISO timestamp of the most recent adoption.
    pub adopted_at: String,
    /// Stable finding IDs (the u8a `finding_id` scheme), sorted + deduped for
    /// stable diffs.
    pub finding_ids: Vec<String>,
}

/// Load the gate baseline. `Ok(None)` when no baseline file exists yet —
/// callers must keep the no-baseline fail-closed behavior.
pub fn load_baseline(path: &Path) -> Result<Option<Baseline>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes).with_context(|| {
            format!("failed to parse baseline: {}", path.display())
        })?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => {
            Err(anyhow::anyhow!(e).context(format!("failed to read baseline: {}", path.display())))
        }
    }
}

/// Write/refresh the baseline from a current scan's finding IDs (the `--adopt`
/// flow). IDs are deduped and sorted so the persisted file diffs stably.
pub fn persist_baseline(path: &Path, finding_ids: &[String]) -> Result<Baseline> {
    let mut ids: Vec<String> = finding_ids.to_vec();
    ids.sort();
    ids.dedup();
    let baseline = Baseline {
        schema_version: 1,
        adopted_at: now_iso8601(),
        finding_ids: ids,
    };
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create baseline dir: {}", parent.display()))?;
        }
    }
    fs::write(path, serde_json::to_vec_pretty(&baseline)?)
        .with_context(|| format!("failed to write baseline: {}", path.display()))?;
    Ok(baseline)
}
