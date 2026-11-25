//! Runner for the `feedback` subcommand: repository extraction and issue
//! creation through genesis.
//!
//! # Responsibilities
//! `FeedbackArgs::run`, the genesis outcome-to-exit-code translation, the
//! CLI-to-genesis kind mapping, the issue-outcome printer, and the
//! Cargo.toml repository extraction (URL trimming in `repo_slug`).
//!
//! # Rationale
//! Split out of `cli/runs.rs` (pretender-5zo.15.14) along the feedback
//! seam so every file stays under the file-size gate. Bodies moved
//! verbatim except where a function had to split further (`run`'s outcome
//! match became `feedback_exit_code`, `extract_repo_from_cargo_toml`'s
//! URL trimming became `repo_slug`) with output unchanged.
use crate::cli::args::{FeedbackArgs, FeedbackKind};
use crate::cli::Executable;
use anyhow::{Context, Result};
use genesis::feedback::{self as genesis_feedback, FeedbackArgs as GenesisFeedbackArgs};
use std::process::ExitCode;

impl Executable for FeedbackArgs {
    fn run(&self) -> Result<ExitCode> {
        let repo = extract_repo_from_cargo_toml()?;
        let project_root = std::env::current_dir().context("failed to get current directory")?;

        let genesis_args = GenesisFeedbackArgs::new(
            feedback_kind_str(self.kind),
            self.dry_run,
            self.from_last_error,
        );
        feedback_exit_code(genesis_feedback::handle_feedback(
            &genesis_args,
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            &repo,
            &project_root,
        ))
    }
}

/// Translate the genesis feedback outcome into a process exit code,
/// printing the created issue or the failure message.
fn feedback_exit_code(result: Result<genesis_feedback::gh::GhResult, String>) -> Result<ExitCode> {
    match result {
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
    Ok(repo_slug(package))
}

/// Convert a repository URL to `owner/repo` form.
///
/// Handles: https://github.com/owner/repo, https://github.com/owner/repo.git,
///          git@github.com:owner/repo.git, ssh://git@github.com/owner/repo.git
fn repo_slug(url: &str) -> String {
    url.trim_start_matches("https://github.com/")
        .trim_start_matches("git@github.com:")
        .trim_start_matches("ssh://git@github.com/")
        .trim_end_matches(".git")
        .to_string()
}
