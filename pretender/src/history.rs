use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const RETENTION_DAYS: u64 = 90;
const MAX_BYTES: u64 = 10 * 1024 * 1024;
const HOTSPOT_MIN_COUNT: usize = 3;
const HOTSPOT_MIN_DAYS: usize = 2;
const PATTERN_MIN_COUNT: usize = 5;
const PATTERN_MIN_FILES: usize = 3;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ViolationEvent {
    pub schema_version: u32,
    pub timestamp: String,
    pub run_id: String,
    pub mode: String,
    pub path: String,
    pub unit_name: Option<String>,
    pub role: String,
    pub rule_key: String,
    pub metric_family: String,
    pub scope: String,
    pub severity: String,
    pub actual: f64,
    pub limit: f64,
    pub delta: f64,
    pub fingerprint: String,
}

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
    let resolution_rate = if previous.is_none() || (fixed.is_empty() && still_open.is_empty()) {
        None
    } else {
        let tracked = fixed.len() + still_open.len();
        Some(fixed.len() as f64 / (tracked.max(1) as f64))
    };
    FindingDelta {
        fixed,
        still_open,
        new,
        resolution_rate,
    }
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
        if let Some(parent) = self.snapshot_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create history dir: {}", parent.display()))?;
        }
        let tmp = self.snapshot_path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(snap)?)?;
        fs::rename(&tmp, &self.snapshot_path)
            .with_context(|| format!("failed to write snapshot: {}", self.snapshot_path.display()))
    }

    /// Loads existing events, appends new_events, prunes by age (RETENTION_DAYS)
    /// and by size (MAX_BYTES keeping the newest half), writes atomically, and
    /// returns the retained set.
    ///
    /// Not safe for concurrent callers sharing the same working directory — last
    /// writer wins. Acceptable for V1 pre-commit / CI best-effort use.
    pub fn append_and_prune(&self, new_events: &[ViolationEvent]) -> Result<Vec<ViolationEvent>> {
        if let Some(parent) = self.events_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create history dir: {}", parent.display()))?;
        }

        let mut events = self.load_events()?;
        events.extend_from_slice(new_events);

        // Age-based retention: drop events with unparseable timestamps (treat as expired).
        let cutoff = now_unix_secs().saturating_sub(RETENTION_DAYS * 86400);
        events.retain(|e| {
            parse_timestamp_secs(&e.timestamp)
                .map(|secs| secs >= cutoff)
                .unwrap_or(false)
        });

        // Keep oldest-first so size-cap trim always discards the oldest half.
        events.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Size cap: estimate bytes in memory to avoid a double disk write.
        let byte_estimate: usize = events
            .iter()
            .map(|e| serde_json::to_string(e).map(|s| s.len() + 1).unwrap_or(0))
            .sum();
        if byte_estimate as u64 > MAX_BYTES {
            let keep_from = events.len() / 2;
            events = events[keep_from..].to_vec();
        }

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

    // Atomic write: write to a sibling .tmp file then rename so a mid-write
    // kill cannot corrupt the event log.
    fn write_events(&self, events: &[ViolationEvent]) -> Result<()> {
        let mut tmp_name = self.events_path.as_os_str().to_owned();
        tmp_name.push(".tmp");
        let tmp_path = PathBuf::from(tmp_name);

        let result = (|| -> Result<()> {
            let file = fs::File::create(&tmp_path)
                .with_context(|| format!("failed to create temp events: {}", tmp_path.display()))?;
            let mut writer = BufWriter::new(file);
            for event in events {
                writeln!(writer, "{}", serde_json::to_string(event)?)?;
            }
            writer.flush()?;
            fs::rename(&tmp_path, &self.events_path).with_context(|| {
                format!("failed to finalize events: {}", self.events_path.display())
            })
        })();

        if result.is_err() {
            let _ = fs::remove_file(&tmp_path);
        }
        result
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

pub fn compute_summary(events: &[ViolationEvent]) -> HistorySummary {
    // Hotspots: same fingerprint, 3+ occurrences across 2+ distinct days
    let mut hotspot_map: HashMap<&str, (usize, HashSet<u64>)> = HashMap::new();
    for event in events {
        let entry = hotspot_map.entry(&event.fingerprint).or_default();
        entry.0 += 1;
        if let Some(secs) = parse_timestamp_secs(&event.timestamp) {
            entry.1.insert(secs / 86400);
        }
    }
    let mut hotspots: Vec<HotspotSummary> = hotspot_map
        .into_iter()
        .filter(|(_, (count, days))| *count >= HOTSPOT_MIN_COUNT && days.len() >= HOTSPOT_MIN_DAYS)
        .map(|(fp, (count, days))| HotspotSummary {
            fingerprint: fp.to_string(),
            count,
            distinct_days: days.len(),
        })
        .collect();
    hotspots.sort_by_key(|b| std::cmp::Reverse(b.count));
    hotspots.truncate(10);

    // Patterns: same (rule_key, role, area), 5+ occurrences across 3+ distinct files
    type PatternKey = (String, String, String);
    let mut pattern_map: HashMap<PatternKey, (usize, HashSet<String>, HashSet<u64>)> =
        HashMap::new();
    for event in events {
        let area = path_area(&event.path);
        let key = (event.rule_key.clone(), event.role.clone(), area);
        let entry = pattern_map.entry(key).or_default();
        entry.0 += 1;
        entry.1.insert(event.path.clone());
        if let Some(secs) = parse_timestamp_secs(&event.timestamp) {
            entry.2.insert(secs / 86400);
        }
    }
    let mut patterns: Vec<PatternSummary> = pattern_map
        .into_iter()
        .filter(|(_, (count, files, _))| {
            *count >= PATTERN_MIN_COUNT && files.len() >= PATTERN_MIN_FILES
        })
        .map(
            |((rule_key, role, area), (count, files, days))| PatternSummary {
                rule_key,
                role,
                area,
                count,
                distinct_files: files.len(),
                distinct_days: days.len(),
            },
        )
        .collect();
    patterns.sort_by_key(|b| std::cmp::Reverse(b.count));
    patterns.truncate(10);

    HistorySummary {
        top_hotspots: hotspots,
        top_patterns: patterns,
        resolution: None,
    }
}

pub fn make_run_id() -> String {
    format!("{}-{}", now_unix_secs(), std::process::id())
}

pub fn now_iso8601() -> String {
    unix_to_iso8601(now_unix_secs())
}

pub fn now_unix_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn unix_to_iso8601(secs: u64) -> String {
    let days = secs / 86400;
    let time = secs % 86400;
    let h = time / 3600;
    let m = (time % 3600) / 60;
    let s = time % 60;
    let (year, month, day) = days_to_ymd(days as i64);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, h, m, s
    )
}

fn parse_timestamp_secs(ts: &str) -> Option<u64> {
    iso8601_to_unix(ts)
}

fn iso8601_to_unix(s: &str) -> Option<u64> {
    if s.len() < 19 || !s.is_ascii() {
        return None;
    }
    let year: i64 = s[0..4].parse().ok()?;
    let month: u64 = s[5..7].parse().ok()?;
    let day: u64 = s[8..10].parse().ok()?;
    let hour: u64 = s[11..13].parse().ok()?;
    let min: u64 = s[14..16].parse().ok()?;
    let sec: u64 = s[17..19].parse().ok()?;
    let days = ymd_to_days(year, month, day)?;
    Some(days * 86400 + hour * 3600 + min * 60 + sec)
}

// Civil date from days since 1970-01-01.
// Algorithm: https://howardhinnant.github.io/date_algorithms.html
fn days_to_ymd(days: i64) -> (u32, u32, u32) {
    let z = days + 719468;
    let era: i64 = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as u32, m as u32, d as u32)
}

fn ymd_to_days(year: i64, month: u64, day: u64) -> Option<u64> {
    let y = if month <= 2 { year - 1 } else { year };
    let era: i64 = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let result = era * 146097 + doe as i64 - 719468;
    if result < 0 {
        None
    } else {
        Some(result as u64)
    }
}

// Extract up to 2 directory components as the area label for pattern grouping,
// e.g. "src/parser" for "src/parser/module.rs". Skips ".." and "." components.
// Absolute paths are not supported; CLI paths are expected to be relative.
fn path_area(path: &str) -> String {
    let p = Path::new(path);
    let dir = match p.parent() {
        Some(d) if !d.as_os_str().is_empty() => d,
        _ => return ".".to_string(),
    };
    let mut parts: Vec<&str> = Vec::new();
    for component in dir.components() {
        if let std::path::Component::Normal(s) = component {
            if let Some(s) = s.to_str() {
                parts.push(s);
                if parts.len() >= 2 {
                    break;
                }
            }
        }
    }
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    fn make_event(
        fingerprint: &str,
        path: &str,
        rule_key: &str,
        role: &str,
        timestamp: &str,
    ) -> ViolationEvent {
        ViolationEvent {
            schema_version: 1,
            timestamp: timestamp.to_string(),
            run_id: "test".to_string(),
            mode: "tiered".to_string(),
            path: path.to_string(),
            unit_name: Some("func".to_string()),
            role: role.to_string(),
            rule_key: rule_key.to_string(),
            metric_family: "complexity".to_string(),
            scope: "unit".to_string(),
            severity: "red".to_string(),
            actual: 20.0,
            limit: 15.0,
            delta: 5.0,
            fingerprint: fingerprint.to_string(),
        }
    }

    #[test]
    fn test_finding_id_uses_unit_name_not_line() {
        // Ticket u8a: the finding ID must be built from path + unit name +
        // rule — not line numbers — so edits that shift lines do not orphan
        // tracked findings.
        let id = finding_id("src/a.rs", "parse", "cognitive_max");
        assert_eq!(id, "src/a.rs::parse::cognitive_max");
        assert!(
            !id.chars().any(|c| c.is_ascii_digit()),
            "no line numbers in ID: {id}"
        );
        assert_eq!(
            finding_id("./src/a.rs", "parse", "cognitive_max"),
            "./src/a.rs::parse::cognitive_max"
        );
    }

    #[test]
    fn test_compute_delta_fixed_still_open_new() {
        let prev = vec![
            "a::f::cog".to_string(),
            "b::g::nest".to_string(),
            "c::h::params".to_string(),
        ];
        let cur = vec!["a::f::cog".to_string(), "d::i::lines".to_string()];
        let delta = compute_delta(Some(&prev), &cur);
        let mut fixed = delta.fixed.clone();
        let mut new = delta.new.clone();
        fixed.sort();
        new.sort();
        assert_eq!(fixed, vec!["b::g::nest", "c::h::params"]);
        assert_eq!(delta.still_open, vec!["a::f::cog"]);
        assert_eq!(new, vec!["d::i::lines"]);
        assert!(
            (delta.resolution_rate.unwrap() - 2.0 / 3.0).abs() < 1e-9,
            "2 fixed / 3 tracked"
        );
    }

    #[test]
    fn test_compute_delta_no_previous_is_all_new() {
        let delta = compute_delta(None, &["a::f::cog".to_string()]);
        assert_eq!(delta.fixed.len(), 0);
        assert_eq!(delta.still_open.len(), 0);
        assert_eq!(delta.new.len(), 1);
        assert!(delta.resolution_rate.is_none(), "no tracked history yet");
    }

    #[test]
    fn test_compute_delta_clean_run_fixes_everything() {
        let delta = compute_delta(Some(&["a::f::cog".to_string()]), &[]);
        assert_eq!(delta.fixed, vec!["a::f::cog"]);
        assert!(delta.still_open.is_empty());
        assert!((delta.resolution_rate.unwrap() - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_snapshot_round_trip() {
        let dir = env::temp_dir().join(format!("pretender-snap-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let store = EventStore::new(&dir);
        assert!(
            store.load_last_snapshot().unwrap().is_none(),
            "no snapshot yet"
        );
        let snap = FindingsSnapshot {
            schema_version: 1,
            run_id: "r1".into(),
            timestamp: "2026-09-29T00:00:00Z".into(),
            finding_ids: vec!["a::f::cog".into()],
            first_seen: Default::default(),
        };
        store.persist_last_snapshot(&snap).expect("persist");
        let loaded = store
            .load_last_snapshot()
            .unwrap()
            .expect("snapshot exists");
        assert_eq!(loaded.finding_ids, vec!["a::f::cog"]);
    }

    #[test]
    fn test_hotspot_meets_threshold() {
        let events = vec![
            make_event(
                "a::func::cog",
                "a.rs",
                "cognitive_max",
                "app",
                "2026-01-01T00:00:00Z",
            ),
            make_event(
                "a::func::cog",
                "a.rs",
                "cognitive_max",
                "app",
                "2026-01-02T00:00:00Z",
            ),
            make_event(
                "a::func::cog",
                "a.rs",
                "cognitive_max",
                "app",
                "2026-01-03T00:00:00Z",
            ),
        ];
        let summary = compute_summary(&events);
        assert_eq!(summary.top_hotspots.len(), 1);
        assert_eq!(summary.top_hotspots[0].count, 3);
        assert_eq!(summary.top_hotspots[0].distinct_days, 3);
    }

    #[test]
    fn test_hotspot_single_day_not_flagged() {
        let ts = "2026-01-01T00:00:00Z";
        let events = vec![
            make_event("a::func::cog", "a.rs", "cognitive_max", "app", ts),
            make_event("a::func::cog", "a.rs", "cognitive_max", "app", ts),
            make_event("a::func::cog", "a.rs", "cognitive_max", "app", ts),
        ];
        let summary = compute_summary(&events);
        assert_eq!(summary.top_hotspots.len(), 0);
    }

    #[test]
    fn test_hotspot_below_count_not_flagged() {
        let events = vec![
            make_event(
                "a::func::cog",
                "a.rs",
                "cognitive_max",
                "app",
                "2026-01-01T00:00:00Z",
            ),
            make_event(
                "a::func::cog",
                "a.rs",
                "cognitive_max",
                "app",
                "2026-01-02T00:00:00Z",
            ),
        ];
        let summary = compute_summary(&events);
        assert_eq!(summary.top_hotspots.len(), 0);
    }

    #[test]
    fn test_pattern_meets_threshold() {
        let ts = "2026-01-01T00:00:00Z";
        let events = vec![
            make_event("fp1", "src/a.rs", "cognitive_max", "app", ts),
            make_event("fp2", "src/b.rs", "cognitive_max", "app", ts),
            make_event("fp3", "src/c.rs", "cognitive_max", "app", ts),
            make_event("fp4", "src/d.rs", "cognitive_max", "app", ts),
            make_event("fp5", "src/e.rs", "cognitive_max", "app", ts),
        ];
        let summary = compute_summary(&events);
        assert_eq!(summary.top_patterns.len(), 1);
        assert_eq!(summary.top_patterns[0].count, 5);
        assert_eq!(summary.top_patterns[0].distinct_files, 5);
    }

    #[test]
    fn test_pattern_below_files_not_flagged() {
        let ts = "2026-01-01T00:00:00Z";
        let events = vec![
            make_event("fp1", "src/a.rs", "cognitive_max", "app", ts),
            make_event("fp2", "src/b.rs", "cognitive_max", "app", ts),
            make_event("fp3", "src/a.rs", "cognitive_max", "app", ts),
            make_event("fp4", "src/b.rs", "cognitive_max", "app", ts),
            make_event("fp5", "src/a.rs", "cognitive_max", "app", ts),
        ];
        let summary = compute_summary(&events);
        assert_eq!(summary.top_patterns.len(), 0);
    }

    #[test]
    fn test_retention_prunes_old_events() {
        let tmp = env::temp_dir().join(format!("pretender-hist-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        let store = EventStore::new(&tmp);

        let old = make_event(
            "fp1",
            "a.rs",
            "cognitive_max",
            "app",
            "1970-01-01T00:00:01Z",
        );
        let recent_ts = unix_to_iso8601(now_unix_secs() - 3600);
        let recent = make_event("fp2", "b.rs", "cognitive_max", "app", &recent_ts);

        let kept = store.append_and_prune(&[old, recent]).unwrap();
        fs::remove_dir_all(&tmp).ok();

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].fingerprint, "fp2");
    }

    #[test]
    fn test_bad_timestamp_event_is_pruned() {
        let tmp = env::temp_dir().join(format!("pretender-hist-bad-ts-{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();
        let store = EventStore::new(&tmp);

        let bad_ts = make_event("fp1", "a.rs", "cognitive_max", "app", "not-a-timestamp");
        let recent_ts = unix_to_iso8601(now_unix_secs() - 3600);
        let recent = make_event("fp2", "b.rs", "cognitive_max", "app", &recent_ts);

        let kept = store.append_and_prune(&[bad_ts, recent]).unwrap();
        fs::remove_dir_all(&tmp).ok();

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].fingerprint, "fp2");
    }

    #[test]
    fn test_iso8601_roundtrip() {
        let ts = "2026-06-10T14:30:00Z";
        let secs = iso8601_to_unix(ts).unwrap();
        assert_eq!(unix_to_iso8601(secs), ts);
    }

    #[test]
    fn test_iso8601_boundary_dates() {
        // Unix epoch
        assert_eq!(unix_to_iso8601(0), "1970-01-01T00:00:00Z");
        // Y2K leap day
        let ts = "2000-02-29T12:00:00Z";
        assert_eq!(unix_to_iso8601(iso8601_to_unix(ts).unwrap()), ts);
        // Century non-leap (2100 is not a leap year)
        let ts = "2100-02-28T00:00:00Z";
        assert_eq!(unix_to_iso8601(iso8601_to_unix(ts).unwrap()), ts);
        // Near Unix i32 max
        let ts = "2038-01-19T03:14:07Z";
        assert_eq!(unix_to_iso8601(iso8601_to_unix(ts).unwrap()), ts);
    }

    #[test]
    fn test_iso8601_rejects_non_ascii() {
        assert!(iso8601_to_unix("2026-06-10T14:30:0\u{00e9}Z").is_none());
    }

    #[test]
    fn test_path_area_extracts_two_components() {
        assert_eq!(path_area("src/parser/module.rs"), "src/parser");
        assert_eq!(path_area("src/main.rs"), "src");
        assert_eq!(path_area("main.rs"), ".");
        assert_eq!(path_area("./src/foo.rs"), "src");
    }
}
