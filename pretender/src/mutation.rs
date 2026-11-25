use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::mutation_entry::{
    build_mutmut_report, enumerate_mutation_sites, parse_cargo_mutants_outcomes,
    parse_stryker_report,
};

// ── language detection ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MutationLang {
    Python,
    Rust,
    JavaScript,
    TypeScript,
    Java,
}

impl MutationLang {
    pub fn tool_name(&self) -> &'static str {
        match self {
            MutationLang::Python => "mutmut",
            MutationLang::Rust => "cargo-mutants",
            MutationLang::JavaScript | MutationLang::TypeScript => "stryker",
            MutationLang::Java => "pitest",
        }
    }
}

pub fn detect_language(path: &Path) -> Option<MutationLang> {
    match path.extension()?.to_str()? {
        "py" => Some(MutationLang::Python),
        "rs" => Some(MutationLang::Rust),
        "js" | "jsx" | "mjs" | "cjs" => Some(MutationLang::JavaScript),
        "ts" | "tsx" | "mts" | "cts" => Some(MutationLang::TypeScript),
        "java" => Some(MutationLang::Java),
        _ => None,
    }
}

/// Return the most-represented language across `paths`, or None if none recognised.
pub fn primary_lang(paths: &[PathBuf]) -> Option<MutationLang> {
    let mut counts: HashMap<MutationLang, usize> = HashMap::new();
    for p in paths {
        if let Some(lang) = detect_language(p) {
            *counts.entry(lang).or_default() += 1;
        }
    }
    counts.into_iter().max_by_key(|(_, c)| *c).map(|(l, _)| l)
}

// ── data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedMutant {
    pub file: String,
    pub line: u32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SurvivingMutant {
    pub file: String,
    pub line: u32,
    pub description: String,
    pub missed_tests: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MutationReport {
    pub tool: String,
    pub total: u32,
    pub killed: u32,
    pub survived: u32,
    pub score_pct: f64,
    pub survivors: Vec<SurvivingMutant>,
}

impl MutationReport {
    pub fn passes_threshold(&self, min_pct: u32) -> bool {
        self.score_pct >= min_pct as f64
    }
}

pub fn compute_score(total: u32, survived: u32) -> f64 {
    if total == 0 {
        return 100.0;
    }
    let killed = total.saturating_sub(survived);
    (killed as f64 / total as f64) * 100.0
}

// ── cargo-mutants parsers ─────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
struct CargoMutantsListEntry {
    #[serde(default)]
    file: String,
    #[serde(default)]
    line: u32,
    #[serde(default)]
    function_name: String,
    #[serde(default)]
    text: String,
}

/// Parse `cargo mutants --list --json` output (JSON array or JSONL).
pub fn parse_cargo_mutants_list(output: &str) -> Vec<PlannedMutant> {
    // Try JSON array first, then fall back to JSONL
    let entries: Vec<CargoMutantsListEntry> =
        if let Ok(arr) = serde_json::from_str::<Vec<CargoMutantsListEntry>>(output) {
            arr
        } else {
            output
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect()
        };

    entries
        .into_iter()
        .map(|e| {
            let description = if e.function_name.is_empty() {
                e.text.clone()
            } else {
                format!("{}: {}", e.function_name, e.text)
            };
            PlannedMutant {
                file: e.file,
                line: e.line,
                description,
            }
        })
        .collect()
}

// ── public API (invokes real tools) ──────────────────────────────────────────

/// List planned mutants for dry-run.
pub fn list_mutants(lang: &MutationLang, paths: &[PathBuf]) -> Result<Vec<PlannedMutant>> {
    match lang {
        MutationLang::Rust => {
            let mut cmd = Command::new("cargo");
            cmd.args(["mutants", "--list", "--json"]);
            for p in paths {
                cmd.args(["--file", &p.to_string_lossy()]);
            }
            let output = cmd
                .output()
                .context("failed to launch `cargo mutants`; is cargo-mutants installed?")?;
            Ok(parse_cargo_mutants_list(&String::from_utf8_lossy(
                &output.stdout,
            )))
        }
        _ => {
            // Use tree-sitter enumeration for all other languages
            let mut all = Vec::new();
            for path in paths {
                if detect_language(path).as_ref() == Some(lang) {
                    if let Ok(source) = std::fs::read_to_string(path) {
                        all.extend(enumerate_mutation_sites(path, &source));
                    }
                }
            }
            Ok(all)
        }
    }
}

/// Run the mutation testing tool and return a normalised report.
pub fn run_mutation(lang: &MutationLang, paths: &[PathBuf]) -> Result<MutationReport> {
    match lang {
        MutationLang::Rust => run_cargo_mutants(paths),
        MutationLang::Python => run_mutmut(paths),
        MutationLang::JavaScript | MutationLang::TypeScript => run_stryker(paths),
        MutationLang::Java => anyhow::bail!(
            "pitest (Java) runner not yet implemented; run `mvn org.pitest:pitest-maven:mutationCoverage` manually"
        ),
    }
}

fn run_cargo_mutants(paths: &[PathBuf]) -> Result<MutationReport> {
    let mut cmd = Command::new("cargo");
    cmd.args(["mutants"]);
    for p in paths {
        cmd.args(["--file", &p.to_string_lossy()]);
    }
    cmd.output()
        .context("failed to launch `cargo mutants`; is cargo-mutants installed?")?;

    let outcomes = PathBuf::from("mutants.out").join("outcomes.json");
    let json = std::fs::read_to_string(&outcomes)
        .context("mutants.out/outcomes.json not found after cargo-mutants run")?;
    parse_cargo_mutants_outcomes(&json)
}

fn run_mutmut(paths: &[PathBuf]) -> Result<MutationReport> {
    let mut run_cmd = Command::new("mutmut");
    run_cmd.arg("run");
    for p in paths {
        run_cmd.arg(p);
    }
    let run_out = run_cmd
        .output()
        .context("failed to launch `mutmut`; is mutmut installed?")?;
    let summary_text = String::from_utf8_lossy(&run_out.stdout).into_owned();

    let results_out = Command::new("mutmut")
        .arg("results")
        .output()
        .context("failed to run `mutmut results`")?;
    let results_text = String::from_utf8_lossy(&results_out.stdout).into_owned();

    Ok(build_mutmut_report(&summary_text, &results_text))
}

fn run_stryker(paths: &[PathBuf]) -> Result<MutationReport> {
    let mut cmd = Command::new("npx");
    cmd.args(["stryker", "run"]);
    // Stryker reads stryker.config.json; pass files as mutate pattern if provided
    if !paths.is_empty() {
        let pattern = paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(",");
        cmd.args(["--mutate", &pattern]);
    }
    cmd.output()
        .context("failed to launch `npx stryker`; is @stryker-mutator/core installed?")?;

    let report_path = PathBuf::from("reports")
        .join("mutation")
        .join("mutation.json");
    let json = std::fs::read_to_string(&report_path)
        .context("reports/mutation/mutation.json not found after stryker run")?;
    parse_stryker_report(&json)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mutation_entry::parse_mutmut_summary;
    use std::path::PathBuf;

    // ── language detection ──────────────────────────────────────────────────

    #[test]
    fn detect_python() {
        assert_eq!(
            detect_language(Path::new("foo.py")),
            Some(MutationLang::Python)
        );
    }

    #[test]
    fn detect_rust() {
        assert_eq!(
            detect_language(Path::new("src/lib.rs")),
            Some(MutationLang::Rust)
        );
    }

    #[test]
    fn detect_javascript() {
        assert_eq!(
            detect_language(Path::new("app.js")),
            Some(MutationLang::JavaScript)
        );
        assert_eq!(
            detect_language(Path::new("index.mjs")),
            Some(MutationLang::JavaScript)
        );
    }

    #[test]
    fn detect_typescript() {
        assert_eq!(
            detect_language(Path::new("foo.ts")),
            Some(MutationLang::TypeScript)
        );
        assert_eq!(
            detect_language(Path::new("bar.tsx")),
            Some(MutationLang::TypeScript)
        );
    }

    #[test]
    fn detect_java() {
        assert_eq!(
            detect_language(Path::new("Foo.java")),
            Some(MutationLang::Java)
        );
    }

    #[test]
    fn detect_unknown_returns_none() {
        assert_eq!(detect_language(Path::new("README.md")), None);
        assert_eq!(detect_language(Path::new("Makefile")), None);
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

    // ── compute_score ───────────────────────────────────────────────────────

    #[test]
    fn score_all_killed() {
        assert!((compute_score(10, 0) - 100.0).abs() < 0.01);
    }

    #[test]
    fn score_none_killed() {
        assert!((compute_score(10, 10) - 0.0).abs() < 0.01);
    }

    #[test]
    fn score_partial() {
        // 8 killed out of 10 = 80%
        assert!((compute_score(10, 2) - 80.0).abs() < 0.01);
    }

    #[test]
    fn score_zero_total() {
        assert!((compute_score(0, 0) - 100.0).abs() < 0.01);
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

    // ── cargo-mutants list parser ───────────────────────────────────────────

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

    // ── cargo-mutants outcomes parser ───────────────────────────────────────

    #[test]
    fn parse_cargo_outcomes_mixed() {
        let json = r#"[
            {"mutant": {"file": "src/lib.rs", "line": 5, "function_name": "foo", "text": "replace"}, "outcome": "caught"},
            {"mutant": {"file": "src/lib.rs", "line": 10, "function_name": "bar", "text": "negate"}, "outcome": "missed"},
            {"mutant": {"file": "src/lib.rs", "line": 15, "function_name": "baz", "text": "remove"}, "outcome": "timeout"}
        ]"#;
        let report = parse_cargo_mutants_outcomes(json)
            .unwrap_or_else(|e| panic!("valid outcomes json: {e}"));
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

    // ── mutmut summary parser ───────────────────────────────────────────────

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

    // ── stryker report parser ───────────────────────────────────────────────

    #[test]
    fn parse_stryker_survived_and_killed() {
        let json = r#"{
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
        let report = parse_stryker_report(json).unwrap_or_else(|e| panic!("valid stryker report: {e}"));
        assert_eq!(report.tool, "stryker");
        assert_eq!(report.total, 2);
        assert_eq!(report.survived, 1);
        assert_eq!(report.killed, 1);
        assert!((report.score_pct - 50.0).abs() < 0.01);
        assert_eq!(report.survivors[0].file, "src/foo.js");
        assert_eq!(report.survivors[0].line, 5);
        assert!(report.survivors[0].description.contains("BinaryOperator"));
    }

    #[test]
    fn parse_stryker_skips_no_coverage_and_compile_error() {
        let json = r#"{
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
        let report =
            parse_stryker_report(json).unwrap_or_else(|e| panic!("valid stryker report: {e}"));
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
        let report =
            parse_stryker_report(json).unwrap_or_else(|e| panic!("valid stryker report: {e}"));
        assert_eq!(report.survivors[0].file, "src/a.js");
        assert_eq!(report.survivors[1].file, "src/b.js");
    }

    #[test]
    fn parse_stryker_invalid_json() {
        assert!(parse_stryker_report("not json").is_err());
    }

    // ── tree-sitter dry-run ─────────────────────────────────────────────────

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
        let source = std::fs::read_to_string(&path).expect("fixture missing");
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
        let source = std::fs::read_to_string(&path).expect("fixture missing");
        let sites = enumerate_mutation_sites(&path, &source);
        let lines: Vec<u32> = sites.iter().map(|s| s.line).collect();
        let mut sorted = lines.clone();
        sorted.sort();
        assert_eq!(lines, sorted, "mutation sites should be sorted by line");
    }
}
