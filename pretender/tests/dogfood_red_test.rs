//! Tests for the `just dogfood-red` recipe (baseline ledger helper).
//!
//! Ledger model: `finding_ids` in a baseline JSON is the list of grandfathered
//! findings. `dogfood-red` un-grandfathers findings by REMOVING exactly the IDs
//! whose path-part (first `::` field) and rule-part (last `::` field) match the
//! given arguments; it can never add IDs. These tests run the real `just`
//! recipe against temporary copies of the committed baseline; the committed
//! `pretender-baseline.json` itself is never mutated.

use std::path::{Path, PathBuf};
use std::process::Command;

const BASELINE_NAME: &str = "pretender-baseline.json";
const TEST_PATH: &str = "pretender/src/main.rs";
const TEST_RULE: &str = "function_lines_max";

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| panic!("pretender package lives inside the repo root"))
        .to_path_buf()
}

fn committed_baseline_bytes() -> Vec<u8> {
    std::fs::read(repo_root().join(BASELINE_NAME))
        .unwrap_or_else(|e| panic!("committed baseline is readable: {e}"))
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

/// Runs the recipe on a temp copy of the committed baseline and returns
/// (original, filtered, matched count, the documented run ID).
fn run_dogfood_red_on_temp_copy() -> (serde_json::Value, serde_json::Value, usize, String) {
    let root = repo_root();
    let original_bytes = committed_baseline_bytes();
    let original: serde_json::Value = serde_json::from_slice(&original_bytes)
        .unwrap_or_else(|e| panic!("committed baseline parses: {e}"));

    let matched = count_matching(&original, (TEST_PATH, TEST_RULE));
    assert!(matched > 0, "fixture must contain matching IDs to remove");
    let run_id = format!("{TEST_PATH}::run::{TEST_RULE}");
    assert!(
        original["finding_ids"]
            .as_array()
            .unwrap_or_else(|| panic!("finding_ids is an array"))
            .iter()
            .any(|id| id.as_str() == Some(run_id.as_str())),
        "fixture must contain the documented {run_id} ID"
    );

    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let baseline = dir.path().join(BASELINE_NAME);
    std::fs::write(&baseline, &original_bytes)
        .unwrap_or_else(|e| panic!("write temp baseline copy: {e}"));

    let output = Command::new("just")
        .arg("--justfile")
        .arg(root.join("justfile"))
        .args(["dogfood-red", TEST_PATH, TEST_RULE])
        .arg(&baseline)
        .current_dir(&root)
        .output()
        .unwrap_or_else(|e| panic!("run just dogfood-red: {e}"));

    assert!(
        output.status.success(),
        "recipe must succeed with an explicit baseline argument; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let filtered_bytes =
        std::fs::read(&baseline).unwrap_or_else(|e| panic!("read filtered baseline: {e}"));
    let filtered: serde_json::Value = serde_json::from_slice(&filtered_bytes)
        .unwrap_or_else(|e| panic!("filtered baseline parses: {e}"));
    (original, filtered, matched, run_id)
}

#[test]
fn dogfood_red_removes_exactly_matching_ids_from_temp_copy() {
    let (original, filtered, matched, run_id) = run_dogfood_red_on_temp_copy();

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
        count_matching(&filtered, (TEST_PATH, TEST_RULE)),
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
            !(id_path == TEST_PATH && id_rule == TEST_RULE)
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

    assert!(
        !actual.contains(&run_id.as_str()),
        "the documented {run_id} ID must be removed"
    );
}

#[test]
fn dogfood_red_fails_cleanly_on_missing_baseline() {
    let root = repo_root();
    let dir = tempfile::tempdir().unwrap_or_else(|e| panic!("tempdir: {e}"));
    let missing = dir.path().join("does-not-exist.json");

    let output = Command::new("just")
        .arg("--justfile")
        .arg(root.join("justfile"))
        .args(["dogfood-red", TEST_PATH, TEST_RULE])
        .arg(&missing)
        .current_dir(&root)
        .output()
        .unwrap_or_else(|e| panic!("run just dogfood-red: {e}"));

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
