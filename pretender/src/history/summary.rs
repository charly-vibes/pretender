//! Purpose: post-run summary scoring over retained violation events.
//! Responsibilities: hotspot and pattern aggregation (thresholds, ordering,
//! top-10 truncation), directory-area labeling, the HistorySummary shape.
//! Rationale: split out of src/history.rs (wave-4 ratchet,
//! pretender-5zo.15.4); scoring decomposed into accumulate/rank stage
//! helpers with identical semantics.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::clock::parse_timestamp_secs;
use super::events::ViolationEvent;

const HOTSPOT_MIN_COUNT: usize = 3;
const HOTSPOT_MIN_DAYS: usize = 2;
const PATTERN_MIN_COUNT: usize = 5;
const PATTERN_MIN_FILES: usize = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionStats {
    pub fixed: usize,
    pub still_open: usize,
    pub new: usize,
    /// fixed / (fixed + still_open); null before the second tracked run.
    pub rate: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotspotSummary {
    pub fingerprint: String,
    pub count: usize,
    pub distinct_days: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSummary {
    pub rule_key: String,
    pub role: String,
    pub area: String,
    pub count: usize,
    pub distinct_files: usize,
    pub distinct_days: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySummary {
    pub top_hotspots: Vec<HotspotSummary>,
    pub top_patterns: Vec<PatternSummary>,
    /// Resolution tracking (ticket u8a): how the current scan compares to the
    /// previous one. None on the first tracked run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<ResolutionStats>,
}

pub fn compute_summary(events: &[ViolationEvent]) -> HistorySummary {
    HistorySummary {
        top_hotspots: top_hotspots(events),
        top_patterns: top_patterns(events),
        resolution: None,
    }
}

/// Per-fingerprint occurrence count and the distinct days it appeared on.
#[derive(Default)]
struct HotspotAcc {
    count: usize,
    days: HashSet<u64>,
}

fn hotspot_counts(events: &[ViolationEvent]) -> HashMap<&str, HotspotAcc> {
    events.iter().fold(HashMap::new(), |mut acc, event| {
        let entry = acc.entry(&event.measure.fingerprint).or_default();
        entry.count += 1;
        if let Some(secs) = parse_timestamp_secs(&event.origin.timestamp) {
            entry.days.insert(secs / 86400);
        }
        acc
    })
}

// Hotspots: same fingerprint, 3+ occurrences across 2+ distinct days.
fn top_hotspots(events: &[ViolationEvent]) -> Vec<HotspotSummary> {
    let mut hotspots: Vec<HotspotSummary> = hotspot_counts(events)
        .into_iter()
        .filter(|(_, acc)| acc.count >= HOTSPOT_MIN_COUNT && acc.days.len() >= HOTSPOT_MIN_DAYS)
        .map(|(fp, acc)| HotspotSummary {
            fingerprint: fp.to_string(),
            count: acc.count,
            distinct_days: acc.days.len(),
        })
        .collect();
    hotspots.sort_by_key(|b| std::cmp::Reverse(b.count));
    hotspots.truncate(10);
    hotspots
}

/// Pattern grouping key: (rule_key, role, area).
type PatternKey = (String, String, String);

/// Per (rule_key, role, area): occurrence count, distinct files and days.
#[derive(Default)]
struct PatternAcc {
    count: usize,
    files: HashSet<String>,
    days: HashSet<u64>,
}

fn pattern_counts(events: &[ViolationEvent]) -> HashMap<PatternKey, PatternAcc> {
    events.iter().fold(HashMap::new(), |mut acc, event| {
        let area = path_area(&event.finding.path);
        let key = (
            event.finding.rule_key.clone(),
            event.finding.role.clone(),
            area,
        );
        let entry = acc.entry(key).or_default();
        entry.count += 1;
        entry.files.insert(event.finding.path.clone());
        if let Some(secs) = parse_timestamp_secs(&event.origin.timestamp) {
            entry.days.insert(secs / 86400);
        }
        acc
    })
}

// Patterns: same (rule_key, role, area), 5+ occurrences across 3+ distinct files.
fn top_patterns(events: &[ViolationEvent]) -> Vec<PatternSummary> {
    let mut patterns: Vec<PatternSummary> = pattern_counts(events)
        .into_iter()
        .filter(|(_, acc)| acc.count >= PATTERN_MIN_COUNT && acc.files.len() >= PATTERN_MIN_FILES)
        .map(|((rule_key, role, area), acc)| PatternSummary {
            rule_key,
            role,
            area,
            count: acc.count,
            distinct_files: acc.files.len(),
            distinct_days: acc.days.len(),
        })
        .collect();
    patterns.sort_by_key(|b| std::cmp::Reverse(b.count));
    patterns.truncate(10);
    patterns
}

// Extract up to 2 directory components as the area label for pattern grouping,
// e.g. "src/parser" for "src/parser/module.rs". Skips ".." and "." components.
// Absolute paths are not supported; CLI paths are expected to be relative.
fn path_area(path: &str) -> String {
    let p = Path::new(path);
    let dir = p.parent().filter(|d| !d.as_os_str().is_empty());
    let parts = dir.map(first_two_normal_components).unwrap_or_default();
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

fn first_two_normal_components(dir: &Path) -> Vec<&str> {
    dir.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .take(2)
        .collect()
}

#[cfg(test)]
mod tests {
    // path_area is private, so its test stays in-file (no unwrap/expect).
    use super::*;

    #[test]
    fn test_path_area_extracts_two_components() {
        assert_eq!(path_area("src/parser/module.rs"), "src/parser");
        assert_eq!(path_area("src/main.rs"), "src");
        assert_eq!(path_area("main.rs"), ".");
        assert_eq!(path_area("./src/foo.rs"), "src");
    }
}
