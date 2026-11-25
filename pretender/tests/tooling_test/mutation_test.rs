//! Mutation-preview and smell-weighting tests.
//!
//! # Responsibilities
//! Pin `mutation --dry-run` exit policy for files and directories, the
//! smell call-weight effect on ABC metrics surfaced via `check --format json`,
//! and the `mutation` module's public unit surface: language detection,
//! score math, threshold checks, and the cargo-mutants list parser.
//!
//! # Rationale
//! Bodies moved verbatim from `tests/cli_test.rs` (wave-3 split) with
//! expect/unwrap calls converted to sanctioned panic-on-failure forms.
//! The unit tests migrated verbatim from `src/mutation.rs` (5zo.15.7)
//! because their items are reachable from the lib crate.

use crate::common::*;
use pretender::mutation::{
    compute_score, detect_language, parse_cargo_mutants_list, primary_lang, MutationLang,
    MutationReport,
};
use std::path::{Path, PathBuf};

#[test]
fn test_mutation_dry_run_python() {
    let output = pretender_cmd()
        .args([
            "mutation",
            "--dry-run",
            source_fixture("python_simple.py")
                .to_str()
                .unwrap_or_else(|| panic!("source fixture path is not valid UTF-8")),
        ])
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "dry-run should exit 0; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("mutmut"),
        "dry-run should name the tool; got: {stdout}"
    );
    assert!(
        stdout.contains("Planned mutants") || stdout.contains("No mutation sites"),
        "dry-run should list planned mutants; got: {stdout}"
    );
}

#[test]
fn test_mutation_dry_run_directory() {
    // Regression: `mutation --dry-run <dir>` failed with
    // "no supported source files found" because directories were passed
    // to detect_language() which checks file extensions.
    let dir = source_fixture("");
    let output = pretender_cmd()
        .args([
            "mutation",
            "--dry-run",
            dir.to_str()
                .unwrap_or_else(|| panic!("source fixture path is not valid UTF-8")),
        ])
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "mutation --dry-run should succeed on a directory; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_smell_call_weights_elevate_abc() {
    let (_dir, path) = stage_fixture("python_smell_calls.py");
    let output = pretender_cmd()
        .arg("check")
        .arg(&path)
        .arg("--format")
        .arg("json")
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("run pretender: {e:?}"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("invalid json: {e}\nstdout: {stdout}"));
    let units = json["data"]["files"][0]["units"]
        .as_array()
        .unwrap_or_else(|| panic!("units array"));
    let abc_for = |name: &str| -> f64 {
        units
            .iter()
            .find(|u| u["name"] == name)
            .unwrap_or_else(|| panic!("unit {name} not found"))["metrics"]["abc"]
            .as_f64()
            .unwrap_or_else(|| panic!("abc metric for {name}"))
    };
    // smell_eval: A=1, B=0, C=5 (eval weight=5.0) → sqrt(26) ≈ 5.099
    let eval_abc = abc_for("smell_eval");
    assert!(
        (eval_abc - 5.099).abs() < 0.01,
        "smell_eval abc={eval_abc}, expected ≈5.099 (sqrt(26))"
    );
    // smell_exec: A=0, B=0, C=5 (exec weight=5.0) → 5.0
    let exec_abc = abc_for("smell_exec");
    assert!(
        (exec_abc - 5.0).abs() < 0.01,
        "smell_exec abc={exec_abc}, expected 5.0"
    );
    // smell_compile: A=1, B=0, C=3 (compile weight=3.0) → sqrt(10) ≈ 3.162
    let compile_abc = abc_for("smell_compile");
    assert!(
        (compile_abc - 3.162).abs() < 0.01,
        "smell_compile abc={compile_abc}, expected ≈3.162 (sqrt(10))"
    );
}

// ── language detection (moved from src/mutation.rs; table-driven so the
//    per-extension cases don't form lazy test clusters) ──────────────────

#[test]
fn detect_maps_extensions_to_languages() {
    let cases = [
        ("foo.py", MutationLang::Python),
        ("src/lib.rs", MutationLang::Rust),
        ("app.js", MutationLang::JavaScript),
        ("index.mjs", MutationLang::JavaScript),
        ("foo.ts", MutationLang::TypeScript),
        ("bar.tsx", MutationLang::TypeScript),
        ("Foo.java", MutationLang::Java),
    ];
    for (path, expected) in cases {
        assert_eq!(
            detect_language(Path::new(path)),
            Some(expected),
            "extension of {path}"
        );
    }
}

#[test]
fn detect_unknown_returns_none() {
    for path in ["README.md", "Makefile"] {
        assert_eq!(detect_language(Path::new(path)), None, "path: {path}");
    }
}

#[test]
fn primary_lang_picks_most_common() {
    let paths = vec![
        PathBuf::from("a.py"),
        PathBuf::from("b.py"),
        PathBuf::from("c.rs"),
    ];
    assert_eq!(primary_lang(&paths), Some(MutationLang::Python));
}

#[test]
fn primary_lang_empty_paths() {
    assert_eq!(primary_lang(&[]), None);
}

// ── compute_score (moved from src/mutation.rs; table-driven for the same
//    lazy-cluster reason) ─────────────────────────────────────────────────

#[test]
fn compute_score_matches_killed_over_total() {
    let cases = [
        (10, 0, 100.0), // all killed
        (10, 10, 0.0),  // none killed
        (10, 2, 80.0),  // 8 killed out of 10
        (0, 0, 100.0),  // zero total
    ];
    for (total, survived, expected) in cases {
        let score = compute_score(total, survived);
        assert!(
            (score - expected).abs() < 0.01,
            "compute_score({total}, {survived}) = {score}, expected {expected}"
        );
    }
}

#[test]
fn passes_threshold() {
    let report = MutationReport {
        tool: "test".to_string(),
        total: 10,
        killed: 8,
        survived: 2,
        score_pct: 80.0,
        survivors: vec![],
    };
    assert!(report.passes_threshold(60));
    assert!(report.passes_threshold(80));
    assert!(!report.passes_threshold(81));
}

// ── cargo-mutants list parser (moved verbatim from src/mutation.rs) ─────

#[test]
fn parse_cargo_list_json_array() {
    let json = r#"[
            {"file": "src/lib.rs", "line": 10, "function_name": "bar", "text": "replace == with !="},
            {"file": "src/lib.rs", "line": 20, "function_name": "baz", "text": "replace + with -"}
        ]"#;
    let mutants = parse_cargo_mutants_list(json);
    assert_eq!(mutants.len(), 2);
    assert_eq!(mutants[0].file, "src/lib.rs");
    assert_eq!(mutants[0].line, 10);
    assert!(mutants[0].description.contains("bar"));
    assert!(mutants[0].description.contains("replace == with !="));
}

#[test]
fn parse_cargo_list_jsonl() {
    let jsonl = concat!(
        "{\"file\": \"src/a.rs\", \"line\": 5, \"function_name\": \"f\", \"text\": \"replace\"}\n",
        "{\"file\": \"src/b.rs\", \"line\": 7, \"function_name\": \"\", \"text\": \"negate\"}\n",
    );
    let mutants = parse_cargo_mutants_list(jsonl);
    assert_eq!(mutants.len(), 2);
    assert_eq!(mutants[0].file, "src/a.rs");
    // No function name → just the text
    assert_eq!(mutants[1].description, "negate");
}

#[test]
fn parse_cargo_list_empty() {
    let mutants = parse_cargo_mutants_list("[]");
    assert!(mutants.is_empty());
}

#[test]
fn parse_cargo_list_ignores_malformed() {
    let jsonl = "not-json\n{\"file\": \"a.rs\", \"line\": 1, \"text\": \"ok\"}\n";
    let mutants = parse_cargo_mutants_list(jsonl);
    assert_eq!(mutants.len(), 1);
}
