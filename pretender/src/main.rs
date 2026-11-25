//! Binary crate root: module wiring and process dispatch.
//!
//! The CLI surface (args, subcommands, runners, config/sink/persistence
//! infrastructure) lives in the `cli` module; the check pipeline in
//! `check`, history emission in `events`, `init` rendering in `init`, and
//! the report writers plus the report data model in `report`.
//!
//! # Responsibilities
//! Parse-and-dispatch entry point: version fast path, guide setup, typo
//! suggestions, command execution, error-sink reporting, and the error
//! scratch record. `CheckArgs` stays here — flattened from the
//! `cli::args` group structs (`CheckScanArgs`, `CheckReportArgs`) so it
//! stays within the field cap, and both check modules read them.
mod check;
mod cli;
mod events;
mod init;
mod report;

use crate::cli::args::{CheckReportArgs, CheckScanArgs, ModeArg};
use crate::cli::{Cli, Commands, Executable, PRETENDER_COMMANDS};
use clap::Parser;
use genesis::feedback::scratch::{self, ErrorRecord};
use genesis::guide::Guide;
use genesis::status::{StatusContributor, StatusItem, StatusSection};
use genesis::suggestions::SuggestionEngine;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
struct CheckArgs {
    #[command(flatten)]
    scan: CheckScanArgs,
    #[command(flatten)]
    report: CheckReportArgs,
    /// Override config `pretender.mode`
    #[arg(long, value_enum)]
    mode: Option<ModeArg>,
    /// Advisory lease expiry (ISO YYYY-MM-DD) for tiered/guidance modes.
    /// Required for advisory modes; an expired/missing lease fails closed —
    /// renew by pushing the date forward (a visible, reviewable diff).
    #[arg(long, value_name = "DATE")]
    advisory_until: Option<String>,
    /// Path to a JUnit XML report for test-duration analysis
    #[arg(long)]
    test_report: Option<PathBuf>,
    /// Execute the configured test_cmd before analyzing test durations
    #[arg(long)]
    execute: bool,
    /// Adopt every current violation into the gate baseline
    /// (pretender-baseline.json): afterwards `check --mode gate` fails only on
    /// genuinely new findings; known (baselined) violations pass with a
    /// summary line. Refresh by re-running with --adopt. Without a baseline,
    /// gate mode still fails on any violation (fail-closed).
    #[arg(long)]
    adopt: bool,
}

fn main() -> ExitCode {
    // Check for --version --json before clap parsing
    if genesis::cli::maybe_print_version_json(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")) {
        return ExitCode::SUCCESS;
    }

    let mut guide = Guide::builder("pretender", env!("CARGO_PKG_VERSION"))
        .commands(PRETENDER_COMMANDS)
        .build();

    let cli = parse_cli(&mut guide);
    dispatch(&mut guide, cli.command)
}

/// Parse global CLI args, offering typo suggestions for bad subcommands.
fn parse_cli(guide: &mut Guide) -> Cli {
    match Cli::try_parse() {
        Ok(cli) => {
            guide.set_verbosity(cli.verbosity());
            cli
        }
        Err(err) => handle_parse_error(err, guide),
    }
}

/// Suggest similar commands for an unrecognized subcommand, otherwise
/// print clap's error and exit with its status.
fn handle_parse_error(err: clap::error::Error, guide: &mut Guide) -> ! {
    if err.kind() == clap::error::ErrorKind::InvalidSubcommand {
        suggest_similar_command(&err.to_string(), guide);
    }
    err.exit()
}

/// Print the typo suggestion for an unrecognized subcommand and exit;
/// falls through without exiting when no suggestion is found.
fn suggest_similar_command(err_msg: &str, guide: &Guide) {
    let Some(bad_cmd) = extract_bad_subcommand(err_msg) else {
        return;
    };
    let engine = SuggestionEngine::new();
    let Some(suggestion) = engine.suggest_typo(&bad_cmd, guide.registry()) else {
        return;
    };
    print_suggestion(&suggestion, &bad_cmd);
    std::process::exit(2);
}

/// Print a "did you mean" suggestion in the guide's suggestion format.
fn print_suggestion(suggestion: &genesis::suggestions::Suggestion, bad_cmd: &str) {
    eprintln!("error: unrecognized subcommand '{bad_cmd}'");
    eprintln!();
    eprintln!("{}", suggestion.message());
    if let Some(footer) = suggestion.footer() {
        eprintln!("{footer}");
    }
}

/// Run the parsed command; surface any failure through the guide's error
/// sink and record the full argv in the error scratch.
fn dispatch(guide: &mut Guide, command: Commands) -> ExitCode {
    match command.run() {
        Ok(code) => code,
        Err(err) => {
            // ErrorSink prints the error and the `feedback --from-last-error`
            // footer. Its built-in scratch only records the tool name, so we
            // disable it and write a richer record with the full argv (which
            // `feedback --from-last-error` surfaces as `Command: …`).
            let sink = guide.error_sink().with_suggest(false).with_scratch(false);
            let mut stderr = io::stderr();
            sink.handle(err.as_ref(), &mut stderr);
            write_error_scratch();
            ExitCode::FAILURE
        }
    }
}

/// Write the failing command to the error scratch so `feedback
/// --from-last-error` can surface the full argv (not just the tool name).
///
/// ErrorSink's built-in scratch records only `argv = [tool_name]`; this
/// restores the v0.1.0 behavior of capturing `std::env::args()`.
fn write_error_scratch() {
    let record = ErrorRecord {
        ts: unix_epoch_seconds(),
        argv: std::env::args().collect(),
        exit: 1,
        footer: None,
        kind: "Error".to_string(),
    };
    scratch::write_scratch_best_effort("pretender", &record);
}

/// Simple Unix-epoch-seconds timestamp.
fn unix_epoch_seconds() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

/// Extract the bad subcommand name from clap's error message.
///
/// Clap format: "error: unrecognized subcommand 'complxity'"
fn extract_bad_subcommand(msg: &str) -> Option<String> {
    let quote_start = msg.find('\'')?;
    let rest = &msg[quote_start + 1..];
    let quote_end = rest.find('\'')?;
    Some(rest[..quote_end].to_string())
}

/// Pretender's status contributor for the cross-tool dashboard.
pub struct PretenderStatus;

impl StatusContributor for PretenderStatus {
    fn name(&self) -> &'static str {
        "pretender"
    }

    fn status(&self, repo_root: &std::path::Path) -> Result<StatusSection, String> {
        let config_exists = repo_root.join("pretender.toml").exists();
        let items = status_items(repo_root, config_exists);
        let summary = if config_exists {
            "configured".to_string()
        } else {
            "not initialized".to_string()
        };

        Ok(StatusSection::with_items("pretender", summary, items))
    }
}

/// Dashboard items for config presence, git detection, and hook install.
fn status_items(repo_root: &std::path::Path, config_exists: bool) -> Vec<StatusItem> {
    let git_dir = repo_root.join(".git");
    let has_git = git_dir.exists();
    let hook_installed = has_git && git_dir.join("hooks/pre-commit").exists();

    let mut items = Vec::new();

    if config_exists {
        items.push(StatusItem::healthy("config", "pretender.toml found"));
    } else {
        items.push(StatusItem::error("config", "pretender.toml not found"));
    }

    if has_git {
        items.push(StatusItem::healthy("git", "git repository detected"));
    } else {
        items.push(StatusItem::warning("git", "not in a git repository"));
    }

    if hook_installed {
        items.push(StatusItem::healthy("hook", "pre-commit hook installed"));
    }

    items
}
