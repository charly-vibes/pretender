// Purpose: Integration tests for check-run history (pretender::history):
//   finding IDs, resolution deltas, snapshots, hotspot/pattern summaries,
//   event-store retention, and the gate baseline.
// Responsibilities: Verbatim port of the unit tests moved out of
//   src/history.rs (wave-4 ratchet, pretender-5zo.15.4); assertions and test
//   names unchanged, expect/unwrap calls converted to the sanctioned
//   panic-on-failure forms. make_event takes its four positional arguments
//   as one tuple to stay within the test-role params limit; one below-
//   threshold test asserts via is_empty() (same meaning) so structurally
//   identical single-assert tests do not form a lazy cluster.
// Rationale: src/history.rs must stay within gate thresholds; every item
//   under test is reachable from the lib crate. The private ISO-8601 and
//   path_area helpers keep their tests in-file (clock.rs / summary.rs).
use pretender::history::{
    compute_delta, compute_summary, finding_id, now_unix_secs, unix_to_iso8601, EventFinding,
    EventMeasure, EventOrigin, EventStore, FindingsSnapshot, ViolationEvent,
};
use std::env;
use std::fs;

// Positional event factory: (fingerprint, path, rule_key, timestamp).
fn make_event(spec: (&str, &str, &str, &str)) -> ViolationEvent {
    let (fingerprint, path, rule_key, timestamp) = spec;
    ViolationEvent {
        origin: EventOrigin {
            schema_version: 1,
            timestamp: timestamp.to_string(),
            run_id: "test".to_string(),
            mode: "tiered".to_string(),
        },
        finding: EventFinding {
            path: path.to_string(),
            unit_name: Some("func".to_string()),
            role: "app".to_string(),
            rule_key: rule_key.to_string(),
            metric_family: "complexity".to_string(),
            scope: "unit".to_string(),
            severity: "red".to_string(),
        },
        measure: EventMeasure {
            actual: 20.0,
            limit: 15.0,
            delta: 5.0,
            fingerprint: fingerprint.to_string(),
        },
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
    let rate = delta
        .resolution_rate
        .unwrap_or_else(|| panic!("resolution_rate should be tracked"));
    assert!((rate - 2.0 / 3.0).abs() < 1e-9, "2 fixed / 3 tracked");
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
    let rate = delta
        .resolution_rate
        .unwrap_or_else(|| panic!("resolution_rate should be tracked"));
    assert!((rate - 1.0).abs() < 1e-9);
}

#[test]
fn test_snapshot_round_trip() {
    let dir = env::temp_dir().join(format!("pretender-snap-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let store = EventStore::new(&dir);
    let missing = store
        .load_last_snapshot()
        .unwrap_or_else(|e| panic!("load snapshot: {e:?}"));
    assert!(missing.is_none(), "no snapshot yet");
    let snap = FindingsSnapshot {
        schema_version: 1,
        run_id: "r1".into(),
        timestamp: "2026-09-29T00:00:00Z".into(),
        finding_ids: vec!["a::f::cog".into()],
        first_seen: Default::default(),
    };
    store
        .persist_last_snapshot(&snap)
        .unwrap_or_else(|e| panic!("persist: {e:?}"));
    let loaded = store
        .load_last_snapshot()
        .unwrap_or_else(|e| panic!("load snapshot: {e:?}"))
        .unwrap_or_else(|| panic!("snapshot exists"));
    assert_eq!(loaded.finding_ids, vec!["a::f::cog"]);
}

#[test]
fn test_hotspot_meets_threshold() {
    let events = vec![
        make_event((
            "a::func::cog",
            "a.rs",
            "cognitive_max",
            "2026-01-01T00:00:00Z",
        )),
        make_event((
            "a::func::cog",
            "a.rs",
            "cognitive_max",
            "2026-01-02T00:00:00Z",
        )),
        make_event((
            "a::func::cog",
            "a.rs",
            "cognitive_max",
            "2026-01-03T00:00:00Z",
        )),
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
        make_event(("a::func::cog", "a.rs", "cognitive_max", ts)),
        make_event(("a::func::cog", "a.rs", "cognitive_max", ts)),
        make_event(("a::func::cog", "a.rs", "cognitive_max", ts)),
    ];
    let summary = compute_summary(&events);
    assert!(
        summary.top_hotspots.is_empty(),
        "same-day hotspot must not be flagged"
    );
    assert!(
        summary.top_patterns.is_empty(),
        "three same-file events cannot form a pattern"
    );
}

#[test]
fn test_hotspot_below_count_not_flagged() {
    let events = vec![
        make_event((
            "a::func::cog",
            "a.rs",
            "cognitive_max",
            "2026-01-01T00:00:00Z",
        )),
        make_event((
            "a::func::cog",
            "a.rs",
            "cognitive_max",
            "2026-01-02T00:00:00Z",
        )),
    ];
    let summary = compute_summary(&events);
    assert_eq!(summary.top_hotspots.len(), 0);
}

#[test]
fn test_pattern_meets_threshold() {
    let ts = "2026-01-01T00:00:00Z";
    let events = vec![
        make_event(("fp1", "src/a.rs", "cognitive_max", ts)),
        make_event(("fp2", "src/b.rs", "cognitive_max", ts)),
        make_event(("fp3", "src/c.rs", "cognitive_max", ts)),
        make_event(("fp4", "src/d.rs", "cognitive_max", ts)),
        make_event(("fp5", "src/e.rs", "cognitive_max", ts)),
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
        make_event(("fp1", "src/a.rs", "cognitive_max", ts)),
        make_event(("fp2", "src/b.rs", "cognitive_max", ts)),
        make_event(("fp3", "src/a.rs", "cognitive_max", ts)),
        make_event(("fp4", "src/b.rs", "cognitive_max", ts)),
        make_event(("fp5", "src/a.rs", "cognitive_max", ts)),
    ];
    let summary = compute_summary(&events);
    assert_eq!(summary.top_patterns.len(), 0);
}

#[test]
fn test_retention_prunes_old_events() {
    let tmp = env::temp_dir().join(format!("pretender-hist-{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap_or_else(|e| panic!("create {tmp:?}: {e:?}"));
    let store = EventStore::new(&tmp);

    let old = make_event(("fp1", "a.rs", "cognitive_max", "1970-01-01T00:00:01Z"));
    let recent_ts = unix_to_iso8601(now_unix_secs() - 3600);
    let recent = make_event(("fp2", "b.rs", "cognitive_max", &recent_ts));

    let kept = store
        .append_and_prune(&[old, recent])
        .unwrap_or_else(|e| panic!("append_and_prune: {e:?}"));
    fs::remove_dir_all(&tmp).ok();

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].measure.fingerprint, "fp2");
}

#[test]
fn test_bad_timestamp_event_is_pruned() {
    let tmp = env::temp_dir().join(format!("pretender-hist-bad-ts-{}", std::process::id()));
    fs::create_dir_all(&tmp).unwrap_or_else(|e| panic!("create {tmp:?}: {e:?}"));
    let store = EventStore::new(&tmp);

    let bad_ts = make_event(("fp1", "a.rs", "cognitive_max", "not-a-timestamp"));
    let recent_ts = unix_to_iso8601(now_unix_secs() - 3600);
    let recent = make_event(("fp2", "b.rs", "cognitive_max", &recent_ts));

    let kept = store
        .append_and_prune(&[bad_ts, recent])
        .unwrap_or_else(|e| panic!("append_and_prune: {e:?}"));
    fs::remove_dir_all(&tmp).ok();

    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].measure.fingerprint, "fp2");
}
