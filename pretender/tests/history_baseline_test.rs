// Purpose: Integration tests for the gate baseline (pretender::history
//   Baseline, persist_baseline / load_baseline).
// Responsibilities: Verbatim port of the baseline unit tests moved out of
//   src/history.rs (wave-4 ratchet, pretender-5zo.15.4); assertions and test
//   names unchanged, expect/unwrap calls converted to the sanctioned
//   panic-on-failure forms. Split from history_test.rs so both files stay
//   within the file-lines target.
// Rationale: src/history.rs must stay within gate thresholds; the baseline
//   API is reachable from the lib crate.
use pretender::history::{load_baseline, persist_baseline};

#[test]
fn test_baseline_roundtrip_persist_load() {
    // gently-if4: persist_baseline then load_baseline round-trips the same
    // finding IDs, deduped and sorted for stable diffs.
    let dir = tempfile::TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e:?}"));
    let path = dir.path().join("pretender-baseline.json");
    let ids = vec![
        "src/b.rs::g::function_lines_max".to_string(),
        "src/a.rs::parse::cognitive_max".to_string(),
        "src/a.rs::parse::cognitive_max".to_string(),
    ];
    let written = persist_baseline(&path, &ids).unwrap_or_else(|e| panic!("persist: {e:?}"));
    assert_eq!(written.finding_ids.len(), 2, "deduped");
    let mut expected = vec![
        "src/a.rs::parse::cognitive_max".to_string(),
        "src/b.rs::g::function_lines_max".to_string(),
    ];
    expected.sort();
    assert_eq!(written.finding_ids, expected, "sorted");
    let loaded = load_baseline(&path)
        .unwrap_or_else(|e| panic!("load: {e:?}"))
        .unwrap_or_else(|| panic!("baseline exists"));
    assert_eq!(loaded.finding_ids, written.finding_ids);
    assert_eq!(loaded.schema_version, 1);
}

#[test]
fn test_load_baseline_missing_file_is_none() {
    // Fail-closed contract: a missing baseline is Ok(None), never an error,
    // so the caller keeps the no-baseline gate behavior.
    let dir = tempfile::TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e:?}"));
    let path = dir.path().join("pretender-baseline.json");
    let loaded = load_baseline(&path).unwrap_or_else(|e| panic!("load: {e:?}"));
    assert!(loaded.is_none());
}

#[test]
fn test_load_baseline_corrupt_file_is_error() {
    // A corrupt baseline must NOT silently degrade to "no baseline":
    // gate would then fail on adopted violations (surprise), so it errors
    // instead and the caller fails closed.
    let dir = tempfile::TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e:?}"));
    let path = dir.path().join("pretender-baseline.json");
    std::fs::write(&path, "{ not json").unwrap_or_else(|e| panic!("write junk: {e:?}"));
    assert!(load_baseline(&path).is_err());
}
