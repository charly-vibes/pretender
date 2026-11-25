//! cargo-mutants report parsing.
//!
//! Purpose: turn `mutants.out/outcomes.json` into a `MutationReport`.
//! Responsibilities: deserialize the outcomes schema, tally
//! totals/kills/survivors in entry order. Also hosts `finished_report`,
//! the shared score/sort/assemble tail used by the other tool parsers.
//! Rationale: one module per external tool; tallies and survivor ordering
//! stay byte-identical to the pre-split `mutation_entry.rs` parser.

use anyhow::{Context, Result};
use serde::Deserialize;

use super::{compute_score, MutationReport, SurvivingMutant};

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

/// Shared report tail: score, sort survivors by file then line, assemble.
pub(super) fn finished_report(
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
