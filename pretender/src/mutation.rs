use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

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

#[derive(Deserialize)]
struct CargoMutantsOutcomeEntry {
    #[serde(default)]
    mutant: CargoMutantInfo,
    #[serde(default)]
    outcome: String,
}

#[derive(Deserialize, Default)]
struct CargoMutantInfo {
    #[serde(default)]
    file: String,
    #[serde(default)]
    line: u32,
    #[serde(default)]
    function_name: String,
    #[serde(default)]
    text: String,
}

/// Parse `mutants.out/outcomes.json` from a cargo-mutants run.
pub fn parse_cargo_mutants_outcomes(json: &str) -> Result<MutationReport> {
    let entries: Vec<CargoMutantsOutcomeEntry> =
        serde_json::from_str(json).context("failed to parse cargo-mutants outcomes.json")?;

    let mut total = 0u32;
    let mut killed = 0u32;
    let mut survivors = Vec::new();

    for e in &entries {
        total += 1;
        match e.outcome.as_str() {
            "missed" => {
                let description = if e.mutant.function_name.is_empty() {
                    e.mutant.text.clone()
                } else {
                    format!("{}: {}", e.mutant.function_name, e.mutant.text)
                };
                survivors.push(SurvivingMutant {
                    file: e.mutant.file.clone(),
                    line: e.mutant.line,
                    description,
                    missed_tests: vec![],
                });
            }
            _ => {
                killed += 1;
            }
        }
    }

    let survived = survivors.len() as u32;
    let score_pct = compute_score(total, survived);
    survivors.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));

    Ok(MutationReport {
        tool: "cargo-mutants".to_string(),
        total,
        killed,
        survived,
        score_pct,
        survivors,
    })
}

// ── mutmut parser ─────────────────────────────────────────────────────────────

/// Parse the final progress line from `mutmut run` output.
/// Returns (total, survived).
/// Line format: "N/M  🎉 K  ⏰ T  🤔 U  🙁 S"
pub fn parse_mutmut_summary(text: &str) -> (u32, u32) {
    for line in text.lines() {
        if let Some((total, survived)) = try_parse_mutmut_line(line) {
            return (total, survived);
        }
    }
    (0, 0)
}

fn try_parse_mutmut_line(line: &str) -> Option<(u32, u32)> {
    // Find "N/M" token
    let fraction = line.split_whitespace().find(|t| t.contains('/'))?;
    let mut parts = fraction.split('/');
    let _current: u32 = parts.next()?.parse().ok()?;
    let total: u32 = parts.next()?.parse().ok()?;
    if total == 0 {
        return None;
    }
    // Find 🙁 count
    let survived = survived_after_skull(line).unwrap_or(0);
    Some((total, survived))
}

fn survived_after_skull(line: &str) -> Option<u32> {
    // "🙁 N" where 🙁 is U+1F641 (3 bytes in UTF-8: 0xF0 0x9F 0x99 0x81, actually 4 bytes)
    // We'll search for the emoji and take the next numeric token.
    let skull = "\u{1F641}";
    let pos = line.find(skull)?;
    let after = &line[pos + skull.len()..];
    after.split_whitespace().next()?.parse().ok()
}

/// Build a MutationReport from mutmut outputs.
/// `summary_text`: stdout from `mutmut run` (contains progress line).
/// `results_text`: stdout from `mutmut results` (contains surviving test names).
pub fn build_mutmut_report(summary_text: &str, results_text: &str) -> MutationReport {
    let (total, survived_count) = parse_mutmut_summary(summary_text);
    let killed = total.saturating_sub(survived_count);
    let score_pct = compute_score(total, survived_count);

    // Parse surviving mutants from `mutmut results` output.
    // Format: blocks separated by "---- N ----" headers.
    let survivors = parse_mutmut_surviving_tests(results_text, survived_count);

    MutationReport {
        tool: "mutmut".to_string(),
        total,
        killed,
        survived: survived_count,
        score_pct,
        survivors,
    }
}

fn parse_mutmut_surviving_tests(text: &str, survived_count: u32) -> Vec<SurvivingMutant> {
    // mutmut results output groups tests by mutant id:
    //   ---- 1 ----
    //   tests/test_foo.py::test_bar
    //   ---- 2 ----
    //   tests/test_foo.py::test_baz
    let mut survivors = Vec::new();
    let mut current_tests: Vec<String> = Vec::new();
    let mut in_block = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("----") && trimmed.ends_with("----") && trimmed.len() > 8 {
            if in_block && !current_tests.is_empty() {
                survivors.push(SurvivingMutant {
                    file: String::new(),
                    line: 0,
                    description: format!("mutant {} survived", survivors.len() + 1),
                    missed_tests: std::mem::take(&mut current_tests),
                });
            }
            in_block = true;
            current_tests.clear();
        } else if in_block && !trimmed.is_empty() && !trimmed.starts_with("To ") && !trimmed.starts_with("mutmut") {
            current_tests.push(trimmed.to_string());
        }
    }
    if in_block && !current_tests.is_empty() {
        survivors.push(SurvivingMutant {
            file: String::new(),
            line: 0,
            description: format!("mutant {} survived", survivors.len() + 1),
            missed_tests: current_tests,
        });
    }

    // If we parsed fewer blocks than the survived count, fill with unknowns
    while survivors.len() < survived_count as usize {
        survivors.push(SurvivingMutant {
            file: String::new(),
            line: 0,
            description: format!("mutant {} survived", survivors.len() + 1),
            missed_tests: vec![],
        });
    }

    survivors
}

// ── stryker parser ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct StrykerReport {
    #[serde(default)]
    files: HashMap<String, StrykerFile>,
}

#[derive(Deserialize)]
struct StrykerFile {
    #[serde(default)]
    mutants: Vec<StrykerMutant>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StrykerMutant {
    #[serde(default)]
    mutator_name: String,
    #[serde(default)]
    replacement: String,
    #[serde(default)]
    location: StrykerLocation,
    #[serde(default)]
    status: String,
}

#[derive(Deserialize, Default)]
struct StrykerLocation {
    #[serde(default)]
    start: StrykerPos,
}

#[derive(Deserialize, Default)]
struct StrykerPos {
    #[serde(default)]
    line: u32,
}

/// Parse a Stryker `mutation.json` report file.
pub fn parse_stryker_report(json: &str) -> Result<MutationReport> {
    let report: StrykerReport =
        serde_json::from_str(json).context("failed to parse stryker mutation.json")?;

    let mut total = 0u32;
    let mut killed = 0u32;
    let mut survivors = Vec::new();

    for (file_path, file) in &report.files {
        for mutant in &file.mutants {
            match mutant.status.as_str() {
                "CompileError" | "Ignored" | "NoCoverage" => continue,
                _ => {}
            }
            total += 1;
            if mutant.status == "Survived" {
                survivors.push(SurvivingMutant {
                    file: file_path.clone(),
                    line: mutant.location.start.line,
                    description: format!(
                        "{}: replace with {}",
                        mutant.mutator_name, mutant.replacement
                    ),
                    missed_tests: vec![],
                });
            } else {
                killed += 1;
            }
        }
    }

    let survived = survivors.len() as u32;
    let score_pct = compute_score(total, survived);
    survivors.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));

    Ok(MutationReport {
        tool: "stryker".to_string(),
        total,
        killed,
        survived,
        score_pct,
        survivors,
    })
}

// ── tree-sitter dry-run enumeration ──────────────────────────────────────────

fn ts_lang_for_mutation(path: &Path) -> Option<tree_sitter::Language> {
    match path.extension()?.to_str()? {
        "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "js" | "jsx" | "mjs" | "cjs" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "ts" | "mts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" | "cts" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        _ => None,
    }
}

/// Enumerate likely mutation sites in a source file using tree-sitter.
/// Finds binary operators, comparison operators, boolean operators, and return
/// statements — the same sites that mutation tools typically target.
pub fn enumerate_mutation_sites(path: &Path, source: &str) -> Vec<PlannedMutant> {
    let Some(ts_lang) = ts_lang_for_mutation(path) else {
        return vec![];
    };
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&ts_lang).is_err() {
        return vec![];
    }
    let Some(tree) = parser.parse(source.as_bytes(), None) else {
        return vec![];
    };
    let file_str = path.to_string_lossy().into_owned();
    let mut sites = Vec::new();
    collect_mutation_sites(tree.root_node(), source.as_bytes(), &file_str, &mut sites);
    sites.sort_by_key(|s| s.line);
    sites
}

fn collect_mutation_sites(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    out: &mut Vec<PlannedMutant>,
) {
    let kind = node.kind();
    let line = node.start_position().row as u32 + 1;

    match kind {
        // Binary arithmetic / comparison / boolean operators
        "binary_operator" | "comparison_operator" | "boolean_operator"
        | "binary_expression" | "augmented_assignment" => {
            // Find the operator child (usually a single-char or keyword node)
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                let op = child.kind();
                if is_mutation_operator(op) {
                    if let Ok(op_text) = child.utf8_text(source) {
                        out.push(PlannedMutant {
                            file: file.to_string(),
                            line,
                            description: format!("replace operator `{op_text}`"),
                        });
                        break;
                    }
                }
            }
        }
        "return_statement" | "return" => {
            out.push(PlannedMutant {
                file: file.to_string(),
                line,
                description: "replace return value".to_string(),
            });
        }
        _ => {}
    }

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_mutation_sites(child, source, file, out);
    }
}

fn is_mutation_operator(kind: &str) -> bool {
    matches!(
        kind,
        "+" | "-" | "*" | "/" | "%" | "**"
        | "==" | "!=" | "<" | ">" | "<=" | ">="
        | "and" | "or" | "&&" | "||" | "not"
        | "<<" | ">>" | "&" | "|" | "^"
        | "+=" | "-=" | "*=" | "/="
    )
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
            let output = cmd.output().context("failed to launch `cargo mutants`; is cargo-mutants installed?")?;
            Ok(parse_cargo_mutants_list(&String::from_utf8_lossy(&output.stdout)))
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
    cmd.output().context("failed to launch `cargo mutants`; is cargo-mutants installed?")?;

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

    let report_path = PathBuf::from("reports").join("mutation").join("mutation.json");
    let json = std::fs::read_to_string(&report_path)
        .context("reports/mutation/mutation.json not found after stryker run")?;
    parse_stryker_report(&json)
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── language detection ──────────────────────────────────────────────────

    #[test]
    fn detect_python() {
        assert_eq!(detect_language(Path::new("foo.py")), Some(MutationLang::Python));
    }

    #[test]
    fn detect_rust() {
        assert_eq!(detect_language(Path::new("src/lib.rs")), Some(MutationLang::Rust));
    }

    #[test]
    fn detect_javascript() {
        assert_eq!(detect_language(Path::new("app.js")), Some(MutationLang::JavaScript));
        assert_eq!(detect_language(Path::new("index.mjs")), Some(MutationLang::JavaScript));
    }

    #[test]
    fn detect_typescript() {
        assert_eq!(detect_language(Path::new("foo.ts")), Some(MutationLang::TypeScript));
        assert_eq!(detect_language(Path::new("bar.tsx")), Some(MutationLang::TypeScript));
    }

    #[test]
    fn detect_java() {
        assert_eq!(detect_language(Path::new("Foo.java")), Some(MutationLang::Java));
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
        let report = parse_cargo_mutants_outcomes(json).unwrap();
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
        let report = parse_cargo_mutants_outcomes(json).unwrap();
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
        let report = parse_cargo_mutants_outcomes(json).unwrap();
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
        assert!(report.survivors[0].missed_tests.iter().any(|t| t.contains("test_a")));
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
        let report = parse_stryker_report(json).unwrap();
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
        let report = parse_stryker_report(json).unwrap();
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
        let report = parse_stryker_report(json).unwrap();
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
            .unwrap()
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
