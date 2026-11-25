//! Stryker report parsing.
//!
//! Purpose: turn a Stryker `mutation.json` report into a `MutationReport`.
//! Responsibilities: deserialize the camelCase schema, tally per-file
//! counts, skip non-counted statuses.
//! Rationale: one module per external tool; per-file tallies and survivor
//! construction stay byte-identical to the pre-split parser.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

use super::cargo_mutants::finished_report;
use super::{MutationReport, SurvivingMutant};

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
        description: format!(
            "{}: replace with {}",
            mutant.mutator_name, mutant.replacement
        ),
        missed_tests: vec![],
    }
}

/// Stryker statuses that count neither as total nor killed mutants.
fn is_skipped_stryker_status(status: &str) -> bool {
    matches!(status, "CompileError" | "Ignored" | "NoCoverage")
}

/// Tally one file's mutants, skipping non-counted statuses.
fn tally_stryker_mutants(
    file_path: &str,
    mutants: &[StrykerMutant],
) -> (u32, u32, Vec<SurvivingMutant>) {
    let counted: Vec<&StrykerMutant> = mutants
        .iter()
        .filter(|mutant| !is_skipped_stryker_status(&mutant.status))
        .collect();
    let survived: Vec<SurvivingMutant> = counted
        .iter()
        .filter(|mutant| mutant.status == "Survived")
        .map(|mutant| stryker_survivor(file_path, mutant))
        .collect();
    let killed = counted.len() - survived.len();
    (counted.len() as u32, killed as u32, survived)
}

/// Sum per-file tallies across the report's files.
fn tally_stryker_files(report: &StrykerReport) -> (u32, u32, Vec<SurvivingMutant>) {
    let tallies: Vec<(u32, u32, Vec<SurvivingMutant>)> = report
        .files
        .iter()
        .map(|(file_path, file)| tally_stryker_mutants(file_path, &file.mutants))
        .collect();
    let total: u32 = tallies.iter().map(|(total, _, _)| *total).sum();
    let killed: u32 = tallies.iter().map(|(_, killed, _)| *killed).sum();
    let survivors: Vec<SurvivingMutant> = tallies
        .into_iter()
        .flat_map(|(_, _, survivors)| survivors)
        .collect();
    (total, killed, survivors)
}
