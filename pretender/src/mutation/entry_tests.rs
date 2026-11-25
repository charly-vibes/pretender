//! Unit tests for the mutation tool-report parsers and site enumeration
//! (crate-private items).
//!
//! Kept in-src because `mutation`'s parser modules are crate-private: their
//! functions are not reachable from the integration-test target. The
//! tests for the public `mutation` items (detection, scoring, list
//! parsing) moved verbatim to `tests/tooling_test/mutation_test.rs`.

use super::cargo_mutants::parse_cargo_mutants_outcomes;
use super::mutmut::{build_mutmut_report, parse_mutmut_summary};
use super::sites::enumerate_mutation_sites;
use super::stryker::parse_stryker_report;
use std::path::{Path, PathBuf};

// ── cargo-mutants outcomes parser ────────────────────────────────────────

#[test]
fn parse_cargo_outcomes_mixed() {
    let json = r#"[
            {"mutant": {"file": "src/lib.rs", "line": 5, "function_name": "foo", "text": "replace"}, "outcome": "caught"},
            {"mutant": {"file": "src/lib.rs", "line": 10, "function_name": "bar", "text": "negate"}, "outcome": "missed"},
            {"mutant": {"file": "src/lib.rs", "line": 15, "function_name": "baz", "text": "remove"}, "outcome": "timeout"}
        ]"#;
    let report =
        parse_cargo_mutants_outcomes(json).unwrap_or_else(|e| panic!("valid outcomes json: {e}"));
    assert_eq!(report.tool, "cargo-mutants");
    assert_eq!(report.total, 3);
    assert_eq!(report.survived, 1);
    assert_eq!(report.killed, 2);
    assert!((report.score_pct - (2.0 / 3.0 * 100.0)).abs() < 0.01);
    assert_eq!(report.survivors[0].file, "src/lib.rs");
    assert_eq!(report.survivors[0].line, 10);
    assert!(report.survivors[0].description.contains("bar"));
}

#[test]
fn parse_cargo_outcomes_all_caught() {
    let json = r#"[
            {"mutant": {"file": "src/lib.rs", "line": 1, "text": "a"}, "outcome": "caught"},
            {"mutant": {"file": "src/lib.rs", "line": 2, "text": "b"}, "outcome": "caught"}
        ]"#;
    let report =
        parse_cargo_mutants_outcomes(json).unwrap_or_else(|e| panic!("valid outcomes json: {e}"));
    assert_eq!(report.survived, 0);
    assert!((report.score_pct - 100.0).abs() < 0.01);
}

#[test]
fn parse_cargo_outcomes_sorted_by_file_then_line() {
    let json = r#"[
            {"mutant": {"file": "src/b.rs", "line": 1, "text": "x"}, "outcome": "missed"},
            {"mutant": {"file": "src/a.rs", "line": 99, "text": "y"}, "outcome": "missed"},
            {"mutant": {"file": "src/a.rs", "line": 5, "text": "z"}, "outcome": "missed"}
        ]"#;
    let report =
        parse_cargo_mutants_outcomes(json).unwrap_or_else(|e| panic!("valid outcomes json: {e}"));
    assert_eq!(report.survivors[0].file, "src/a.rs");
    assert_eq!(report.survivors[0].line, 5);
    assert_eq!(report.survivors[1].file, "src/a.rs");
    assert_eq!(report.survivors[1].line, 99);
    assert_eq!(report.survivors[2].file, "src/b.rs");
}

#[test]
fn parse_cargo_outcomes_invalid_json() {
    assert!(parse_cargo_mutants_outcomes("not json").is_err());
}

// ── mutmut summary parser ────────────────────────────────────────────────

#[test]
fn parse_mutmut_progress_line() {
    let output = "⠋ 20/20  \u{1F389} 18  \u{23F0} 0  \u{1F914} 0  \u{1F641} 2  \n";
    let (total, survived) = parse_mutmut_summary(output);
    assert_eq!(total, 20);
    assert_eq!(survived, 2);
}

#[test]
fn parse_mutmut_zero_survived() {
    let output = "⠋ 5/5  \u{1F389} 5  \u{23F0} 0  \u{1F914} 0  \u{1F641} 0  \n";
    let (total, survived) = parse_mutmut_summary(output);
    assert_eq!(total, 5);
    assert_eq!(survived, 0);
}

#[test]
fn parse_mutmut_no_match_returns_zeros() {
    let (total, survived) = parse_mutmut_summary("no progress line here");
    assert_eq!(total, 0);
    assert_eq!(survived, 0);
}

#[test]
fn build_mutmut_report_correct_score() {
    let summary = "⠋ 10/10  \u{1F389} 8  \u{23F0} 0  \u{1F914} 0  \u{1F641} 2  \n";
    let results = concat!(
        "---- 1 ----\n",
        "tests/test_foo.py::test_a\n",
        "---- 2 ----\n",
        "tests/test_foo.py::test_b\n",
    );
    let report = build_mutmut_report(summary, results);
    assert_eq!(report.total, 10);
    assert_eq!(report.survived, 2);
    assert_eq!(report.killed, 8);
    assert!((report.score_pct - 80.0).abs() < 0.01);
    assert_eq!(report.survivors.len(), 2);
    assert!(report.survivors[0]
        .missed_tests
        .iter()
        .any(|t| t.contains("test_a")));
}

// ── stryker report parser ────────────────────────────────────────────────

/// Stryker report with one survived and one killed mutant (verbatim fixture).
const STRYKER_SURVIVED_KILLED_JSON: &str = r#"{
            "files": {
                "src/foo.js": {
                    "mutants": [
                        {"mutatorName": "BinaryOperator", "replacement": "!=",
                         "location": {"start": {"line": 5, "column": 3}},
                         "status": "Survived", "killedBy": []},
                        {"mutatorName": "LogicalOperator", "replacement": "||",
                         "location": {"start": {"line": 10, "column": 1}},
                         "status": "Killed", "killedBy": ["test1"]}
                    ]
                }
            }
        }"#;

#[test]
fn parse_stryker_survived_and_killed() {
    let report = parse_stryker_report(STRYKER_SURVIVED_KILLED_JSON)
        .unwrap_or_else(|e| panic!("valid stryker report: {e}"));
    assert_eq!(report.tool, "stryker");
    assert_eq!(report.total, 2);
    assert_eq!(report.survived, 1);
    assert_eq!(report.killed, 1);
    assert!((report.score_pct - 50.0).abs() < 0.01);
    assert_eq!(report.survivors[0].file, "src/foo.js");
    assert_eq!(report.survivors[0].line, 5);
    assert!(report.survivors[0].description.contains("BinaryOperator"));
}

/// Stryker report whose NoCoverage and CompileError mutants are skipped
/// (verbatim fixture).
const STRYKER_SKIPPED_STATUSES_JSON: &str = r#"{
            "files": {
                "src/bar.js": {
                    "mutants": [
                        {"mutatorName": "X", "replacement": "y",
                         "location": {"start": {"line": 1}},
                         "status": "NoCoverage", "killedBy": []},
                        {"mutatorName": "X", "replacement": "y",
                         "location": {"start": {"line": 2}},
                         "status": "CompileError", "killedBy": []},
                        {"mutatorName": "X", "replacement": "y",
                         "location": {"start": {"line": 3}},
                         "status": "Killed", "killedBy": ["t"]}
                    ]
                }
            }
        }"#;

#[test]
fn parse_stryker_skips_no_coverage_and_compile_error() {
    let report = parse_stryker_report(STRYKER_SKIPPED_STATUSES_JSON)
        .unwrap_or_else(|e| panic!("valid stryker report: {e}"));
    assert_eq!(report.total, 1);
    assert_eq!(report.survived, 0);
    assert_eq!(report.killed, 1);
}

#[test]
fn parse_stryker_survivors_sorted() {
    let json = r#"{
            "files": {
                "src/b.js": {"mutants": [
                    {"mutatorName": "A", "replacement": "x",
                     "location": {"start": {"line": 1}}, "status": "Survived", "killedBy": []}
                ]},
                "src/a.js": {"mutants": [
                    {"mutatorName": "B", "replacement": "y",
                     "location": {"start": {"line": 99}}, "status": "Survived", "killedBy": []}
                ]}
            }
        }"#;
    let report = parse_stryker_report(json).unwrap_or_else(|e| panic!("valid stryker report: {e}"));
    assert_eq!(report.survivors[0].file, "src/a.js");
    assert_eq!(report.survivors[1].file, "src/b.js");
}

#[test]
fn parse_stryker_invalid_json() {
    assert!(parse_stryker_report("not json").is_err());
}

// ── tree-sitter dry-run ──────────────────────────────────────────────────

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| panic!("manifest dir has no parent"))
        .join("tests/fixtures")
        .join(name)
}

#[test]
fn enumerate_python_simple_finds_sites() {
    let path = fixture_path("python_simple.py");
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture missing: {e:?}"));
    let sites = enumerate_mutation_sites(&path, &source);
    assert!(
        !sites.is_empty(),
        "expected at least one mutation site in python_simple.py"
    );
    for site in &sites {
        assert!(site.line > 0);
        assert!(!site.description.is_empty());
    }
}

#[test]
fn enumerate_unsupported_extension_returns_empty() {
    let sites = enumerate_mutation_sites(Path::new("foo.txt"), "hello world");
    assert!(sites.is_empty());
}

#[test]
fn enumerate_sites_are_sorted_by_line() {
    let path = fixture_path("python_simple.py");
    let source =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("fixture missing: {e:?}"));
    let sites = enumerate_mutation_sites(&path, &source);
    let lines: Vec<u32> = sites.iter().map(|s| s.line).collect();
    let mut sorted = lines.clone();
    sorted.sort();
    assert_eq!(lines, sorted, "mutation sites should be sorted by line");
}
