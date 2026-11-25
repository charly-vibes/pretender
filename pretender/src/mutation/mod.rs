//! Mutation testing pipeline: language detection and external-tool runners.
//!
//! Purpose: detect the mutation language for a target, invoke the matching
//! tool (`cargo mutants` / `mutmut` / `stryker`), and hand back normalised
//! `MutationReport`s or planned-mutant listings.
//! Responsibilities: extension-based language detection, per-tool command
//! construction, and delegation of output parsing to the per-tool parser
//! modules. Report types and scoring live in `report`; dry-run
//! source-site enumeration in `sites`; the parsers' unit tests in
//! `entry_tests`.

mod cargo_mutants;
mod mutmut;
mod report;
mod sites;
mod stryker;

pub use report::{
    compute_score, parse_cargo_mutants_list, MutationReport, PlannedMutant, SurvivingMutant,
};

use anyhow::{Context, Result};
use cargo_mutants::parse_cargo_mutants_outcomes;
use mutmut::build_mutmut_report;
use sites::enumerate_mutation_sites;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use stryker::parse_stryker_report;

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
    let counts = paths
        .iter()
        .fold(HashMap::<MutationLang, usize>::new(), |mut counts, p| {
            if let Some(lang) = detect_language(p) {
                *counts.entry(lang).or_default() += 1;
            }
            counts
        });
    counts.into_iter().max_by_key(|(_, c)| *c).map(|(l, _)| l)
}

/// `--file` argument pairs for cargo-mutants invocations.
fn cargo_file_args(paths: &[PathBuf]) -> Vec<String> {
    paths
        .iter()
        .flat_map(|p| ["--file".to_string(), p.to_string_lossy().into_owned()])
        .collect()
}

/// Tree-sitter mutation sites for every path matching `lang`.
fn tree_sitter_sites(lang: &MutationLang, paths: &[PathBuf]) -> Vec<PlannedMutant> {
    paths
        .iter()
        .filter(|p| detect_language(p).as_ref() == Some(lang))
        .filter_map(|p| std::fs::read_to_string(p).ok().map(|src| (p, src)))
        .flat_map(|(p, src)| enumerate_mutation_sites(p, &src))
        .collect()
}

// ── public API (invokes real tools) ──────────────────────────────────────────

/// List planned mutants for dry-run.
pub fn list_mutants(lang: &MutationLang, paths: &[PathBuf]) -> Result<Vec<PlannedMutant>> {
    match lang {
        MutationLang::Rust => {
            let file_args = cargo_file_args(paths);
            let output = Command::new("cargo")
                .args(["mutants", "--list", "--json"])
                .args(file_args)
                .output()
                .context("failed to launch `cargo mutants`; is cargo-mutants installed?")?;
            Ok(parse_cargo_mutants_list(&String::from_utf8_lossy(
                &output.stdout,
            )))
        }
        _ => Ok(tree_sitter_sites(lang, paths)),
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
    let file_args = cargo_file_args(paths);
    Command::new("cargo")
        .args(["mutants"])
        .args(file_args)
        .output()
        .context("failed to launch `cargo mutants`; is cargo-mutants installed?")?;

    let outcomes = PathBuf::from("mutants.out").join("outcomes.json");
    let json = std::fs::read_to_string(&outcomes)
        .context("mutants.out/outcomes.json not found after cargo-mutants run")?;
    parse_cargo_mutants_outcomes(&json)
}

fn run_mutmut(paths: &[PathBuf]) -> Result<MutationReport> {
    let run_out = Command::new("mutmut")
        .arg("run")
        .args(paths)
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

/// `--mutate` argument pair for Stryker, empty when no files are given.
fn stryker_mutate_args(paths: &[PathBuf]) -> Vec<String> {
    if paths.is_empty() {
        return Vec::new();
    }
    let pattern = paths
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(",");
    vec!["--mutate".to_string(), pattern]
}

fn run_stryker(paths: &[PathBuf]) -> Result<MutationReport> {
    let mutate = stryker_mutate_args(paths);
    Command::new("npx")
        .args(["stryker", "run"])
        .args(mutate)
        .output()
        .context("failed to launch `npx stryker`; is @stryker-mutator/core installed?")?;

    let report_path = PathBuf::from("reports")
        .join("mutation")
        .join("mutation.json");
    let json = std::fs::read_to_string(&report_path)
        .context("reports/mutation/mutation.json not found after stryker run")?;
    parse_stryker_report(&json)
}

#[cfg(test)]
mod entry_tests;
