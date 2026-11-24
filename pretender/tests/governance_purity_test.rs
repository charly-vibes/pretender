//! Governance purity gate: the baseline freeze window (ticket pretender-5zo.19.2).
//!
//! # Design (human-approved deviation from the literal ticket text)
//!
//! The literal rule — "no baseline ID may live under `pretender/src/` or
//! `pretender/tests/`" — is impossible while the epic's burn-down ledger still
//! holds the adopted production debt, and it would also outlaw ticket .15's
//! sanctioned re-adopt. The approved design instead freezes the current window:
//!
//! * `governance/baseline-freeze.json` is an exact (byte-identical) copy of the
//!   committed `pretender-baseline.json` as of the freeze. It is the burn-down
//!   window this gate measures against.
//! * Rule 1: every production ID baselined under `pretender/src/` or
//!   `pretender/tests/` (an ID's path-part is its first `::`-split element)
//!   must ALSO exist in the freeze. A production ID absent from the freeze is
//!   RED — baselining new production debt is the gaming move.
//! * Rule 2: any non-production ID (path outside `pretender/src/` and
//!   `pretender/tests/`, e.g. `scripts/`, `docs/`, README examples) is
//!   outright RED — non-production debt is never baselineable.
//!
//! The freeze shrinks only: removals are monotone and require review, and
//! ticket .15's sanctioned re-adopt replaces the whole file in ONE deliberate
//! reviewed commit. The literal end-state — no production IDs anywhere — is
//! reached when the epic's ledger empties and .15 re-adopts by replacing the
//! freeze with an empty (or shrinking) window.
//!
//! Hermetic by construction: this gate reads two JSON files and does string
//! checks; it never invokes the pretender binary.

use std::fs;
use std::path::Path;

const BASELINE: &str = "pretender-baseline.json";
const FREEZE: &str = "governance/baseline-freeze.json";
const PRODUCTION_PREFIXES: [&str; 2] = ["pretender/src/", "pretender/tests/"];

type Rule = fn() -> Vec<String>;

/// Each row is (rule label, rule fn returning its RED violations).
const RULES: &[(&str, Rule)] = &[
    (
        "non-production IDs are never baselineable",
        baseline_non_production_ids,
    ),
    (
        "production IDs absent from the governance/baseline-freeze.json window",
        production_ids_missing_from_freeze,
    ),
];

/// `finding_ids` strings of a baseline-shaped JSON file at the repository root.
fn load_ids(relative: &str) -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(relative);
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let doc: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()));
    doc["finding_ids"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: missing finding_ids array", path.display()))
        .iter()
        .map(|id| id.as_str().expect("finding id is a string").to_string())
        .collect()
}

/// An ID's path-part: its first `::`-split element.
fn path_part(id: &str) -> &str {
    id.split("::").next().unwrap_or(id)
}

/// Production paths are the crate's own source and integration tests.
fn is_production_path(path: &str) -> bool {
    PRODUCTION_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(prefix))
}

fn is_production(id: &str) -> bool {
    is_production_path(path_part(id))
}

fn is_in_freeze(id: &str, freeze: &[String]) -> bool {
    freeze.iter().any(|frozen| frozen == id)
}

/// RED set for rule 2: baselined IDs whose path is not production debt.
fn baseline_non_production_ids() -> Vec<String> {
    load_ids(BASELINE)
        .iter()
        .filter(|id| !is_production(id))
        .cloned()
        .collect()
}

/// RED set for rule 1: production debt baselined outside the frozen window.
fn production_ids_missing_from_freeze() -> Vec<String> {
    let freeze = load_ids(FREEZE);
    load_ids(BASELINE)
        .iter()
        .filter(|id| is_production(id) && !is_in_freeze(id, &freeze))
        .cloned()
        .collect()
}

fn violations_of(rule_index: usize) -> (&'static str, Vec<String>) {
    let (label, rule) = RULES[rule_index];
    (label, rule())
}

/// Table row 0: any non-production ID in the baseline is outright RED.
#[test]
fn baseline_holds_no_non_production_ids() {
    let (label, violations) = violations_of(0);
    assert!(
        violations.is_empty(),
        "PURITY RED ({label}): {violations:#?}"
    );
}

/// Table row 1: every production ID must already sit in the frozen window.
#[test]
fn freeze_window_covers_every_production_baseline_id() {
    let (label, violations) = violations_of(1);
    assert!(
        violations.is_empty(),
        "PURITY RED ({label}): {violations:#?}"
    );
}
