//! Mutation report data model and `cargo mutants --list` parsing.
//!
//! Purpose: define the normalised mutation-testing vocabulary shared by the
//! runners and the external-tool parsers: planned/surviving mutants, the
//! scored report, and the percentage math.
//! Responsibilities: hold the serde round-trip types, score computation,
//! and the cargo-mutants list format (JSON array or JSONL).

use serde::{Deserialize, Serialize};

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

// ── cargo-mutants list parser ─────────────────────────────────────────────────

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
    parse_list_entries(output)
        .into_iter()
        .map(entry_to_planned_mutant)
        .collect()
}

/// Read the list entries: a whole-output JSON array when it parses,
/// otherwise JSONL (one entry per non-empty line, unparsable lines dropped).
fn parse_list_entries(output: &str) -> Vec<CargoMutantsListEntry> {
    // Try JSON array first, then fall back to JSONL
    if let Ok(arr) = serde_json::from_str::<Vec<CargoMutantsListEntry>>(output) {
        return arr;
    }
    output
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Convert a raw list entry to a planned mutant, preferring the
/// `function: text` description when a function name is present.
fn entry_to_planned_mutant(e: CargoMutantsListEntry) -> PlannedMutant {
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
}
