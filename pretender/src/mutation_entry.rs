//! Mutation pipeline inputs: external tool reports and source mutation sites.
//!
//! Purpose: turn cargo-mutants/mutmut/Stryker output into `MutationReport`s and
//! enumerate dry-run operator/return mutation sites in source via tree-sitter.
//! Responsibilities: deserialize tool formats, tally totals/kills/survivors,
//! sort survivors by file then line, score, and walk syntax trees.
//! Rationale: split out of `mutation.rs`; oversized parsers were decomposed
//! along tally/summary seams so ordering and walk order stay byte-identical.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::mutation::{compute_score, MutationReport, PlannedMutant, SurvivingMutant};

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
pub(crate) fn parse_cargo_mutants_outcomes(json: &str) -> Result<MutationReport> {
    let entries: Vec<CargoMutantsOutcomeEntry> =
        serde_json::from_str(json).context("failed to parse cargo-mutants outcomes.json")?;
    let (total, killed, survivors) = tally_cargo_outcomes(&entries);
    Ok(finished_report("cargo-mutants", total, killed, survivors))
}

/// The survivor for a "missed" outcome entry, if the entry is one.
fn cargo_outcome_survivor(entry: &CargoMutantsOutcomeEntry) -> Option<SurvivingMutant> {
    if entry.outcome != "missed" {
        return None;
    }
    let description = if entry.mutant.function_name.is_empty() {
        entry.mutant.text.clone()
    } else {
        format!("{}: {}", entry.mutant.function_name, entry.mutant.text)
    };
    Some(SurvivingMutant {
        file: entry.mutant.file.clone(),
        line: entry.mutant.line,
        description,
        missed_tests: vec![],
    })
}

/// Count total/killed entries and collect survivors in entry order.
fn tally_cargo_outcomes(entries: &[CargoMutantsOutcomeEntry]) -> (u32, u32, Vec<SurvivingMutant>) {
    entries.iter().fold(
        (0u32, 0u32, Vec::new()),
        |(mut total, mut killed, mut survivors), entry| {
            total += 1;
            match cargo_outcome_survivor(entry) {
                Some(survivor) => survivors.push(survivor),
                None => killed += 1,
            }
            (total, killed, survivors)
        },
    )
}

/// Shared report tail: score, sort survivors by file then line, assemble.
fn finished_report(
    tool: &str,
    total: u32,
    killed: u32,
    mut survivors: Vec<SurvivingMutant>,
) -> MutationReport {
    let survived = survivors.len() as u32;
    let score_pct = compute_score(total, survived);
    survivors.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    MutationReport {
        tool: tool.to_string(),
        total,
        killed,
        survived,
        score_pct,
        survivors,
    }
}

// ── mutmut parser ─────────────────────────────────────────────────────────────

/// Parse the final progress line from `mutmut run`; returns (total, survived).
/// Line format: "N/M  🎉 K  ⏰ T  🤔 U  🙁 S"
pub(crate) fn parse_mutmut_summary(text: &str) -> (u32, u32) {
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
pub(crate) fn build_mutmut_report(summary_text: &str, results_text: &str) -> MutationReport {
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

/// `mutmut results` blocks as survivors, padded to `survived_count` with unknowns.
fn parse_mutmut_surviving_tests(text: &str, survived_count: u32) -> Vec<SurvivingMutant> {
    let mut survivors = parse_mutmut_blocks(text);
    while survivors.len() < survived_count as usize {
        survivors.push(mutmut_survivor(survivors.len() + 1, vec![]));
    }
    survivors
}

/// One `SurvivingMutant` for mutmut block `index` (1-based) with `tests`.
fn mutmut_survivor(index: usize, tests: Vec<String>) -> SurvivingMutant {
    SurvivingMutant {
        file: String::new(),
        line: 0,
        description: format!("mutant {index} survived"),
        missed_tests: tests,
    }
}

/// Collect one survivor per "---- N ----" block of test names.
fn parse_mutmut_blocks(text: &str) -> Vec<SurvivingMutant> {
    let mut survivors = Vec::new();
    let mut current_tests: Vec<String> = Vec::new();
    let mut in_block = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("----") && trimmed.ends_with("----") && trimmed.len() > 8 {
            if in_block && !current_tests.is_empty() {
                let tests = std::mem::take(&mut current_tests);
                survivors.push(mutmut_survivor(survivors.len() + 1, tests));
            }
            in_block = true;
            current_tests.clear();
        } else if in_block
            && !trimmed.is_empty()
            && !trimmed.starts_with("To ")
            && !trimmed.starts_with("mutmut")
        {
            current_tests.push(trimmed.to_string());
        }
    }
    if in_block && !current_tests.is_empty() {
        survivors.push(mutmut_survivor(survivors.len() + 1, current_tests));
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
pub(crate) fn parse_stryker_report(json: &str) -> Result<MutationReport> {
    let report: StrykerReport =
        serde_json::from_str(json).context("failed to parse stryker mutation.json")?;
    let (total, killed, survivors) = tally_stryker_files(&report);
    Ok(finished_report("stryker", total, killed, survivors))
}

/// One survivor for a Stryker mutant that stayed alive.
fn stryker_survivor(file_path: &str, mutant: &StrykerMutant) -> SurvivingMutant {
    SurvivingMutant {
        file: file_path.to_string(),
        line: mutant.location.start.line,
        description: format!("{}: replace with {}", mutant.mutator_name, mutant.replacement),
        missed_tests: vec![],
    }
}

/// Tally one file's mutants, skipping non-counted statuses.
fn tally_stryker_mutants(
    file_path: &str,
    mutants: &[StrykerMutant],
) -> (u32, u32, Vec<SurvivingMutant>) {
    mutants.iter().fold(
        (0u32, 0u32, Vec::new()),
        |(total, killed, mut survivors), mutant| {
            if matches!(mutant.status.as_str(), "CompileError" | "Ignored" | "NoCoverage") {
                return (total, killed, survivors);
            }
            if mutant.status == "Survived" {
                survivors.push(stryker_survivor(file_path, mutant));
                (total + 1, killed, survivors)
            } else {
                (total + 1, killed + 1, survivors)
            }
        },
    )
}

/// Sum per-file tallies across the report's files.
fn tally_stryker_files(report: &StrykerReport) -> (u32, u32, Vec<SurvivingMutant>) {
    report.files.iter().fold(
        (0u32, 0u32, Vec::new()),
        |(total, killed, mut survivors), (file_path, file)| {
            let (file_total, file_killed, file_survivors) =
                tally_stryker_mutants(file_path, &file.mutants);
            survivors.extend(file_survivors);
            (total + file_total, killed + file_killed, survivors)
        },
    )
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
pub(crate) fn enumerate_mutation_sites(path: &Path, source: &str) -> Vec<PlannedMutant> {
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
    let mut sites = collect_sites(tree.root_node(), source.as_bytes(), &file_str);
    sites.sort_by_key(|s| s.line);
    sites
}

/// The mutation site a single node represents, if any.
fn node_mutation_site(node: tree_sitter::Node, source: &[u8], file: &str) -> Option<PlannedMutant> {
    let line = node.start_position().row as u32 + 1;
    match node.kind() {
        // Binary arithmetic / comparison / boolean operators
        "binary_operator" | "comparison_operator" | "boolean_operator" | "binary_expression"
        | "augmented_assignment" => {
            // Find the operator child (usually a single-char or keyword node)
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if is_mutation_operator(child.kind()) {
                    if let Ok(op_text) = child.utf8_text(source) {
                        return Some(PlannedMutant {
                            file: file.to_string(),
                            line,
                            description: format!("replace operator `{op_text}`"),
                        });
                    }
                }
            }
            None
        }
        "return_statement" | "return" => Some(PlannedMutant {
            file: file.to_string(),
            line,
            description: "replace return value".to_string(),
        }),
        _ => None,
    }
}

/// Depth-first pre-order collection: the node's own site, then its children.
fn collect_sites(node: tree_sitter::Node, source: &[u8], file: &str) -> Vec<PlannedMutant> {
    let own = node_mutation_site(node, source, file);
    let mut cursor = node.walk();
    let children = node
        .named_children(&mut cursor)
        .flat_map(|child| collect_sites(child, source, file));
    own.into_iter().chain(children).collect()
}

fn is_mutation_operator(kind: &str) -> bool {
    matches!(
        kind,
        "+" | "-"
            | "*"
            | "/"
            | "%"
            | "**"
            | "=="
            | "!="
            | "<"
            | ">"
            | "<="
            | ">="
            | "and"
            | "or"
            | "&&"
            | "||"
            | "not"
            | "<<"
            | ">>"
            | "&"
            | "|"
            | "^"
            | "+="
            | "-="
            | "*="
            | "/="
    )
}
