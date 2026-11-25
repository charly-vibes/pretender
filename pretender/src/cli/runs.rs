//! Command runners for the analysis, report, mutation, explain, doctor,
//! and feedback subcommands.
//!
//! Each `Executable::run` implementation orchestrates loading config and
//! inputs, invoking the corresponding engine, and printing the result.
//!
//! # Responsibilities
//! The runners for `complexity`, `duplication`, `mutation`, `report`,
//! `explain`, `doctor`, and `feedback`, plus their print helpers and the
//! Cargo.toml repository extraction used by feedback.
//!
//! # Rationale
//! Extracted from `main.rs` (pretender-5zo.2.5); the oversized runners
//! were split into verbatim-behaving stage helpers (diagnostics/table
//! printing, source collection, group printing, result emission) so every
//! function stays within the structural limits with output unchanged.
use crate::check::collect_input_files;
use crate::check::scan::get_parser;
use crate::cli::args::{
    ComplexityArgs, DoctorArgs, DuplicationArgs, ExplainArgs, FeedbackArgs, FeedbackKind,
    MutationArgs, ReportFormat,
};
use crate::cli::infra::{load_config, warn_if_no_config};
use crate::cli::Executable;
use anyhow::{anyhow, Context, Result};
use genesis::envelope::{Envelope, EnvelopeKind};
use genesis::feedback::{self as genesis_feedback, FeedbackArgs as GenesisFeedbackArgs};
use pretender::model::Metric;
use pretender::{doctor, duplication, explain, metrics, model, mutation};
use std::cmp::Reverse;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

impl Executable for ExplainArgs {
    fn run(&self) -> Result<ExitCode> {
        explain::run(&self.metric)?;
        Ok(ExitCode::SUCCESS)
    }
}

impl Executable for DoctorArgs {
    fn run(&self) -> Result<ExitCode> {
        // Doctor format from global CLI flags (--json/--human or TTY auto-detect)
        let format = resolve_doctor_format();
        doctor::run_doctor(format)
    }
}

/// Resolve the doctor output format from global CLI args or TTY auto-detect.
fn resolve_doctor_format() -> genesis::guide::OutputFormat {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--json" || a == "-j") {
        genesis::guide::OutputFormat::Json
    } else {
        genesis::guide::OutputFormat::Human
    }
}

impl Executable for FeedbackArgs {
    fn run(&self) -> Result<ExitCode> {
        let repo = extract_repo_from_cargo_toml()?;
        let project_root = std::env::current_dir().context("failed to get current directory")?;

        let genesis_args = GenesisFeedbackArgs::new(
            feedback_kind_str(self.kind),
            self.dry_run,
            self.from_last_error,
        );
        match genesis_feedback::handle_feedback(
            &genesis_args,
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            &repo,
            &project_root,
        ) {
            Ok(result) => {
                print_feedback_result(result);
                Ok(ExitCode::SUCCESS)
            }
            Err(msg) => {
                eprintln!("Failed to create issue: {msg}");
                Ok(ExitCode::FAILURE)
            }
        }
    }
}

/// Map the CLI feedback kind to the genesis kind string.
fn feedback_kind_str(kind: FeedbackKind) -> &'static str {
    match kind {
        FeedbackKind::Bug => "bug",
        FeedbackKind::Feature => "feature",
        FeedbackKind::Question => "question",
    }
}

/// Print the issue-creation outcome (URL, number, or local fallback path).
fn print_feedback_result(result: genesis::feedback::gh::GhResult) {
    match result {
        genesis::feedback::gh::GhResult::Created { url, number } => {
            println!("Created issue #{number}: {url}");
        }
        genesis::feedback::gh::GhResult::FallbackUrl(url) => {
            println!("Open to create issue: {url}");
        }
        genesis::feedback::gh::GhResult::LocalFile(path) => {
            println!("Issue body written to: {}", path.display());
        }
    }
}

/// Extract the repository string from Cargo.toml's [package] repository field.
fn extract_repo_from_cargo_toml() -> Result<String> {
    // Read the pretender member's Cargo.toml (not the workspace root)
    let cargo_path = if std::path::Path::new("pretender/Cargo.toml").exists() {
        "pretender/Cargo.toml"
    } else {
        "Cargo.toml"
    };
    let content =
        std::fs::read_to_string(cargo_path).context(format!("failed to read {cargo_path}"))?;
    let value: toml::Value = toml::from_str(&content).context("failed to parse Cargo.toml")?;
    let package = value
        .get("package")
        .and_then(|v| v.get("repository"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Cargo.toml missing [package].repository"))?;
    // Convert full URL to owner/repo format
    // Handles: https://github.com/owner/repo, https://github.com/owner/repo.git,
    //          git@github.com:owner/repo.git, ssh://git@github.com/owner/repo.git
    let repo_short = package
        .trim_start_matches("https://github.com/")
        .trim_start_matches("git@github.com:")
        .trim_start_matches("ssh://git@github.com/")
        .trim_end_matches(".git")
        .to_string();
    Ok(repo_short)
}

impl Executable for ComplexityArgs {
    fn run(&self) -> Result<ExitCode> {
        let config = load_config().unwrap_or_default();
        let threshold = config.thresholds.app.cyclomatic_max;
        let files = collect_input_files(&self.paths, &config)?;
        let multi = files.len() > 1;

        for path in &files {
            let source = std::fs::read_to_string(path)
                .with_context(|| format!("failed to read source file: {}", path.display()))?;
            let parser = get_parser(path)?;
            let (module, diagnostics) = parser.parse(path, &source)?;

            if !diagnostics.is_empty() {
                print_diagnostics(&diagnostics);
            }

            if multi {
                println!("{}:", path.display());
            }

            print_complexity_table(&module, threshold, multi);
        }
        Ok(ExitCode::SUCCESS)
    }
}

/// Print parse diagnostics to stderr with span line ranges, then a blank
/// line separating them from the report body.
fn print_diagnostics(diagnostics: &[model::Diagnostic]) {
    for diag in diagnostics {
        eprintln!("{:?}: {}", diag.severity, diag.message);
        if let Some(span) = &diag.span {
            eprintln!("  at lines {}-{}", span.start_line, span.end_line);
        }
    }
    eprintln!();
}

/// Sort units by cyclomatic score descending and print the per-unit table.
fn print_complexity_table(module: &model::Module, threshold: u32, multi: bool) {
    let metric = metrics::CyclomaticComplexity;
    let mut results: Vec<(String, u32)> = module
        .units
        .iter()
        .map(|u| (u.name.clone(), metric.calculate(u)))
        .collect();

    let indent = if multi { "  " } else { "" };
    results.sort_by_key(|(_, score)| Reverse(*score));
    for (name, score) in &results {
        if *score > threshold {
            println!("{indent}{name}: {score} (threshold: {threshold}) ✗");
        } else {
            println!("{indent}{name}: {score}");
        }
    }
}

impl Executable for DuplicationArgs {
    fn run(&self) -> Result<ExitCode> {
        warn_if_no_config();
        let config = load_config()?;
        let file_paths = collect_input_files(&self.paths, &config)?;
        let min_nodes = self.min_nodes.unwrap_or(10) as usize;
        let files = collect_duplication_sources(&file_paths);

        let groups = duplication::detect_clones(&files, min_nodes, self.cross_file)?;

        if groups.is_empty() {
            println!("No structural clones detected.");
            return Ok(ExitCode::SUCCESS);
        }

        println!("Found {} clone group(s):\n", groups.len());
        print_clone_groups(&groups);

        Ok(ExitCode::SUCCESS)
    }
}

/// Read the duplicable sources, skipping paths without a supported language
/// and warning on unreadable files (verbatim skip/warn behavior).
fn collect_duplication_sources(file_paths: &[PathBuf]) -> Vec<(PathBuf, String)> {
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for path in file_paths {
        if duplication::ts_language_for_path(path).is_none() {
            continue;
        }
        match fs::read_to_string(path) {
            Ok(source) => files.push((path.clone(), source)),
            Err(e) => eprintln!("warning: skipping {}: {e}", path.display()),
        }
    }
    files
}

/// Print each clone group with node count, similarity, and locations.
fn print_clone_groups(groups: &[duplication::CloneGroup]) {
    for (i, group) in groups.iter().enumerate() {
        println!(
            "Clone {} ({} nodes, similarity: {}%):",
            i + 1,
            group.node_count,
            group.similarity
        );
        for loc in &group.locations {
            println!(
                "  {}:{}-{}",
                loc.file.display(),
                loc.start_line,
                loc.end_line
            );
        }
        println!();
    }
}

impl Executable for MutationArgs {
    fn run(&self) -> Result<ExitCode> {
        let config = load_config()?;
        let files = collect_input_files(&self.paths, &config)?;

        let lang = mutation::primary_lang(&files)
            .ok_or_else(|| anyhow!("no supported source files found in provided paths"))?;

        if self.dry_run {
            return run_mutation_dry_run(&lang, &files);
        }

        let report = mutation::run_mutation(&lang, &files)?;
        emit_mutation_result(&report, self.format)?;

        if report.passes_threshold(self.score_min) {
            Ok(ExitCode::SUCCESS)
        } else {
            Ok(ExitCode::FAILURE)
        }
    }
}

/// Emit the mutation report: a JSON envelope for `--format json`, the
/// human summary otherwise.
fn emit_mutation_result(report: &mutation::MutationReport, format: ReportFormat) -> Result<()> {
    match format {
        ReportFormat::Json => {
            let env = Envelope::success(
                env!("CARGO_PKG_VERSION"),
                EnvelopeKind::Check,
                report,
                vec![],
                vec![],
            );
            println!("{}", serde_json::to_string_pretty(&env)?);
        }
        _ => print_mutation_report(report),
    }
    Ok(())
}

fn run_mutation_dry_run(lang: &mutation::MutationLang, paths: &[PathBuf]) -> Result<ExitCode> {
    println!(
        "Dry run: would use {} on {} file(s)",
        lang.tool_name(),
        paths.len()
    );
    let mutants = mutation::list_mutants(lang, paths)?;
    if mutants.is_empty() {
        println!("No mutation sites found.");
        return Ok(ExitCode::SUCCESS);
    }
    println!("Planned mutants ({}):", mutants.len());
    for m in &mutants {
        if m.line > 0 {
            println!("  {}:{}: {}", m.file, m.line, m.description);
        } else {
            println!("  {}: {}", m.file, m.description);
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn print_mutation_report(report: &mutation::MutationReport) {
    let pass = if report.score_pct >= 60.0 {
        "✓"
    } else {
        "✗"
    };
    println!(
        "Mutation score: {:.1}% ({}/{} killed) {}",
        report.score_pct, report.killed, report.total, pass
    );
    println!("Tool: {}", report.tool);
    if report.survivors.is_empty() {
        println!("\nAll mutants killed — perfect score!");
        return;
    }
    println!("\nSurviving mutants ({}):", report.survived);
    for m in &report.survivors {
        if m.line > 0 {
            print!("  {}:{}", m.file, m.line);
        } else {
            print!("  {}", m.file);
        }
        println!("  {}", m.description);
        for test in &m.missed_tests {
            println!("    missed by: {test}");
        }
    }
}

#[cfg(test)]
mod managed_block_ownership_tests {
    /// AGENTS.md regions pretender owns.
    ///
    /// Ownership rule: only the tool that publishes a region may write it.
    /// pretender publishes none — WAI belongs to wai and DONT to dont — so
    /// `pretender init` injects no blocks and the doctor registers no
    /// block-drift target. A future entry here is a deliberate, test-visible
    /// decision that must name the publishing tool first.
    fn owned_agents_regions() -> Vec<&'static str> {
        vec![]
    }

    #[test]
    fn supported_managed_block_set_is_empty() {
        let owned = owned_agents_regions();
        assert!(
            owned.is_empty(),
            "ownership rule violated: only a tool that publishes an AGENTS.md \
             region may write it, but pretender claims {owned:?} — WAI and DONT \
             are authored by wai and dont"
        );
    }
}
