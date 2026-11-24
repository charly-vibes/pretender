//! Governance gates over pretender's own repository (pretender-5zo.19):
//!
//! - .19.1 baseline monotonicity: the committed baseline's finding-ID set
//!   stays a subset of `git show origin/main:pretender-baseline.json` — a
//!   gaming move that ADDS an ID to the baseline to silence a finding fails.
//! - .19.4 excludes: pretender.toml excludes exactly the reviewed fixture set.
//! - .19.5 assertions floor: total test assertions reported by the gate run
//!   reach governance/assertions.count.

#[path = "governance/util.rs"]
mod util;

use serde_json::Value;
use std::path::Path;
use std::process::Command;

fn parse_finding_ids(text: &str, what: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        panic!("failed to parse {what}")
    };
    value["finding_ids"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|id| id.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn baseline_ids(root: &Path, relative: &str) -> Vec<String> {
    parse_finding_ids(&util::read(root, relative), "parse gate baseline JSON")
}

fn git_show_baseline_ids(root: &Path) -> Vec<String> {
    let attempt = Command::new("git")
        .args(["show", "origin/main:pretender-baseline.json"])
        .current_dir(root)
        .output();
    let Ok(output) = attempt else {
        panic!("failed to run git show")
    };
    assert!(
        output.status.success(),
        "git show origin/main:pretender-baseline.json failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    parse_finding_ids(&stdout, "parse origin/main gate baseline JSON")
}

/// .19.1: the committed baseline may only ever shrink against origin/main.
/// Adding an ID would silence a finding without the finding being fixed, so
/// every committed ID must exist on origin/main too.
#[test]
fn gate_baseline_ids_are_a_subset_of_origin_main() {
    let Some(root) = util::repo_root() else {
        return;
    };
    let committed = baseline_ids(&root, "pretender-baseline.json");
    let origin = git_show_baseline_ids(&root);
    let gained: Vec<&String> = committed
        .iter()
        .filter(|id| !origin.contains(*id))
        .collect();
    assert!(
        gained.is_empty(),
        "committed baseline gained IDs absent from origin/main: {gained:?}"
    );
}

fn unit_assertions(unit: &Value) -> i64 {
    unit["metrics"]["assertions"].as_i64().unwrap_or(0)
}

fn total_assertions(report: &Value) -> i64 {
    let Some(files) = report["data"]["files"].as_array() else {
        return 0;
    };
    files
        .iter()
        .filter_map(|file| file["units"].as_array())
        .flatten()
        .map(unit_assertions)
        .sum()
}

fn run_gate_report(root: &Path) -> (bool, Value) {
    let (ok, stdout) = util::hermetic_check(root);
    let Ok(report) = serde_json::from_str::<Value>(&stdout) else {
        panic!("failed to parse check JSON")
    };
    (ok, report)
}

/// The reviewed fixture set (pretender-5zo.19.4): the excludes must equal
/// this set exactly — no sneaking production code out of the gate scope.
const REVIEWED_EXCLUDES: [&str; 4] = [
    "vendor/**",
    "node_modules/**",
    "**/*_generated.*",
    "**/fixtures/**",
];

#[test]
fn gate_excludes_are_exactly_the_reviewed_set() {
    let Some(root) = util::repo_root() else {
        return;
    };
    let Ok(config) = toml::from_str::<toml::Value>(&util::read(&root, "pretender.toml")) else {
        panic!("failed to parse pretender.toml")
    };
    let Some(excludes) = config["pretender"]["exclude"].as_array() else {
        panic!("pretender.toml must define pretender.exclude")
    };
    assert_eq!(
        excludes.len(),
        REVIEWED_EXCLUDES.len(),
        "exclude set must be exactly the reviewed entries"
    );
    for entry in REVIEWED_EXCLUDES {
        let present = excludes.iter().any(|value| value.as_str() == Some(entry));
        assert!(present, "pretender.toml exclude must contain {entry:?}");
    }
}

/// .19.5: the gate run must see at least the committed number of test
/// assertions across all units (hermetic CARGO_BIN_EXE run, JSON sum).
#[test]
fn test_assertions_total_meets_governance_floor() {
    let Some(root) = util::repo_root() else {
        return;
    };
    let (ok, report) = run_gate_report(&root);
    let total = total_assertions(&report);
    let Ok(floor) = util::read(&root, "governance/assertions.count")
        .trim()
        .parse::<i64>()
    else {
        panic!("failed to parse governance/assertions.count")
    };
    assert!(
        total >= floor,
        "test assertions {total} below governance floor {floor} (gate exit ok={ok})"
    );
}

/// 5zo.17: the pre-commit gate must run the checkout build, not a globally
/// installed binary — a stale global would score the tree with old metric
/// semantics and pollute the shared `.pretender` history store. Contract:
/// both the ddl gate source (`.ddl/config.toml`) and the rendered hook
/// (`lefthook.yml`) route through the workspace (cargo run / just recipe).
#[test]
fn pre_commit_gate_resolves_the_checkout_build() {
    let Some(root) = util::repo_root() else {
        return;
    };
    let ddl = util::read(&root, ".ddl/config.toml");
    assert!(
        !ddl.contains("pretender_gate = \"pretender "),
        ".ddl/config.toml gates.pretender_gate must invoke the checkout build, not a bare global binary"
    );
    let hook = util::read(&root, "lefthook.yml");
    let run_line = hook
        .lines()
        .find(|line| line.trim_start().starts_with("run:") && line.contains("--mode gate"))
        .expect("lefthook.yml must keep a ddl-pretender-gate run line");
    let checkout_build = run_line.contains("cargo run") || run_line.contains("just ");
    assert!(
        checkout_build,
        "pre-commit gate run line must use the checkout build (cargo run or just recipe): {run_line:?}"
    );
}
