//! Tests for the `just dogfood-red` recipe (baseline ledger helper).
//!
//! Ledger model: `finding_ids` in a baseline JSON is the list of grandfathered
//! findings. `dogfood-red` un-grandfathers findings by REMOVING exactly the IDs
//! whose path-part (first `::` field) and rule-part (last `::` field) match the
//! given arguments; it can never add IDs. These tests run the real `just`
//! recipe against a FIXTURE baseline: filtering is pure string matching, so the
//! mechanics (exact-ID removal, byte-preservation of the rest) are exercised
//! without coupling to the live ledger — coupling to the committed ledger would
//! break by design once the ledger reaches its target state: empty.

use std::path::{Path, PathBuf};
use std::process::Command;

const BASELINE_NAME: &str = "pretender-baseline.json";

/// Self-contained fixture ledger. The first ID's (path, rule) selects the
/// filter target: exactly one ID matches (`engine.rs :: file_lines`); the
/// other two must survive byte-identically, in order.
const FIXTURE_BASELINE: &str = r#"{
  "schema_version": 1,
  "adopted_at": "2026-11-25T00:00:00Z",
  "finding_ids": [
    "pretender/src/engine.rs::::file_lines",
    "pretender/src/engine.rs::compile_query::cyclomatic_max",
    "pretender/src/history.rs::::file_lines"
  ]
}"#;

/// Picks the selector exercised by the recipe run from the fixture ledger: the
/// first finding ID's path-part and rule-part.
fn pick_selector(baseline: &serde_json::Value) -> (String, String) {
    let first = baseline["finding_ids"]
        .as_array()
        .unwrap_or_else(|| panic!("finding_ids is an array"))
        .first()
        .and_then(|id| id.as_str())
        .unwrap_or_else(|| {
            panic!("fixture ledger must retain at least one finding for the fixture")
        })
        .to_string();
    let parts: Vec<&str> = first.split("::").collect();
    (
        parts
            .first()
            .unwrap_or_else(|| panic!("ID has a path part"))
            .to_string(),
        parts
            .last()
            .unwrap_or_else(|| panic!("ID has a rule part"))
            .to_string(),
    )
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| panic!("pretender package lives inside the repo root"))
        .to_path_buf()
}

fn fixture_baseline_bytes() -> Vec<u8> {
    FIXTURE_BASELINE.as_bytes().to_vec()
}

fn id_parts(id: &str) -> (&str, &str) {
    let parts: Vec<&str> = id.split("::").collect();
    (
        parts.first().copied().unwrap_or(""),
        parts.last().copied().unwrap_or(""),
    )
}

fn count_matching(baseline: &serde_json::Value, selector: (&str, &str)) -> usize {
    let (path, rule) = selector;
    baseline["finding_ids"]
        .as_array()
        .unwrap_or_else(|| panic!("finding_ids is an array"))
        .iter()
        .filter(|id| {
            let (id_path, id_rule) = id_parts(
                id.as_str()
                    .unwrap_or_else(|| panic!("finding_ids are strings")),
            );
            id_path == path && id_rule == rule
        })
        .count()
}

/// Writes a temp copy of the ledger `bytes` into `dir` and returns its path.
fn write_baseline_copy(dir: &tempfile::TempDir, bytes: &[u8]) -> PathBuf {
    let baseline = dir.path().join(BASELINE_NAME);
    std::fs::write(&baseline, bytes).unwrap_or_else(|e| panic!("write temp baseline copy: {e}"));
    baseline
}

/// Runs `just dogfood-red <path> <rule>` on `baseline` from the repo root.
fn run_dogfood_red(baseline: &Path, selector: (&str, &str)) -> std::process::Output {
    let (test_path, test_rule) = selector;
    let root = repo_root();
    Command::new("just")
        .arg("--justfile")
        .arg(root.join("justfile"))
        .args(["dogfood-red", test_path, test_rule])
        .arg(baseline)
        .current_dir(&root)
        .output()
        .unwrap_or_else(|e| panic!("run just dogfood-red: {e}"))
}

/// Runs the recipe on a temp copy of the fixture baseline and returns
/// (original, filtered, matched count, the (path, rule) selector used).
fn run_dogfood_red_on_temp_copy() -> (
    serde_json::Value,
    serde_json::Value,
    usize,
    (String, String),
) {
    let original_bytes = fixture_baseline_bytes();
    let original: serde_json::Value = serde_json::from_slice(&original_bytes)
        .unwrap_or_else(|e| panic!("fixture baseline parses: {e}"));

    let (test_path, test_rule) = pick_selector(&original);
    let matched = count_matching(&original, (&test_path, &test_rule));
    assert!(matched > 0, "fixture must contain matching IDs to remove");

    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let baseline = write_baseline_copy(&dir, &original_bytes);

    let output = run_dogfood_red(&baseline, (&test_path, &test_rule));
    assert!(
        output.status.success(),
        "recipe must succeed with an explicit baseline argument; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let filtered_bytes =
        std::fs::read(&baseline).unwrap_or_else(|e| panic!("read filtered baseline: {e}"));
    let filtered: serde_json::Value = serde_json::from_slice(&filtered_bytes)
        .unwrap_or_else(|e| panic!("filtered baseline parses: {e}"));
    (original, filtered, matched, (test_path, test_rule))
}

#[test]
fn dogfood_red_removes_exactly_matching_ids_from_temp_copy() {
    let (original, filtered, matched, (test_path, test_rule)) = run_dogfood_red_on_temp_copy();

    assert_eq!(
        filtered["schema_version"], original["schema_version"],
        "schema_version must be preserved"
    );
    assert_eq!(
        filtered["adopted_at"], original["adopted_at"],
        "adopted_at must be preserved"
    );

    let before = original["finding_ids"]
        .as_array()
        .unwrap_or_else(|| panic!("finding_ids is an array"));
    let after = filtered["finding_ids"]
        .as_array()
        .unwrap_or_else(|| panic!("finding_ids is an array"));
    assert_eq!(
        after.len(),
        before.len() - matched,
        "exactly the {matched} matching IDs must be removed"
    );
    assert_eq!(
        count_matching(&filtered, (&test_path, &test_rule)),
        0,
        "no matching ID may survive the filter"
    );

    // Non-matching IDs are byte-preserved, in their original order.
    let expected: Vec<&str> = before
        .iter()
        .map(|id| {
            id.as_str()
                .unwrap_or_else(|| panic!("finding_ids are strings"))
        })
        .filter(|id| {
            let (id_path, id_rule) = id_parts(id);
            !(id_path == test_path && id_rule == test_rule)
        })
        .collect();
    let actual: Vec<&str> = after
        .iter()
        .map(|id| {
            id.as_str()
                .unwrap_or_else(|| panic!("finding_ids are strings"))
        })
        .collect();
    assert_eq!(
        actual, expected,
        "remaining IDs must be the original non-matching IDs, byte-identical and in order"
    );
}

#[test]
fn dogfood_red_fails_cleanly_on_missing_baseline() {
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let missing = dir.path().join("does-not-exist.json");

    let (test_path, test_rule) = pick_selector(
        &serde_json::from_slice(&fixture_baseline_bytes())
            .unwrap_or_else(|e| panic!("fixture baseline parses: {e}")),
    );

    let output = run_dogfood_red(&missing, (&test_path, &test_rule));

    assert!(
        !output.status.success(),
        "recipe must exit non-zero when the baseline file does not exist"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("baseline") && stderr.contains("not found"),
        "error must clearly name the missing baseline file, got: {stderr}"
    );
}
