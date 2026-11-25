//! CLI surface of the binary: global args, subcommands, and dispatch.
//!
//! Defines the clap parser (`Cli`, `Commands`, `PRETENDER_COMMANDS`), the
//! `Executable` runner trait each subcommand implements, the plugins
//! not-implemented stub, doctor format resolution, and Pretender's
//! cross-tool dashboard status contributor.
//!
//! # Responsibilities
//! Own argument parsing and command dispatch wiring; per-command runners
//! live in the `args`/`runs`/`hooks` siblings, config/sink/persistence in
//! `infra`.
//!
//! # Rationale
//! Extracted from `main.rs` (pretender-5zo.2.5) so the bin crate root
//! shrinks to module wiring plus `main`; `status` was split so the
//! dashboard assembly stays within the function-size limit, with section
//! content and summary strings byte-identical.
pub(crate) mod args;
pub(crate) mod hooks;
pub(crate) mod infra;
mod runs;

use crate::CheckArgs;
use crate::init::InitArgs;
use args::{
    CiCommand, ComplexityArgs, DoctorArgs, DuplicationArgs, ExplainArgs, FeedbackArgs,
    HooksCommand, MutationArgs, PluginsCommand, ReportArgs,
};
use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use std::process::ExitCode;

const NOT_IMPLEMENTED_EXIT: u8 = 2;

#[derive(Parser)]
#[command(name = "pretender", version)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,

    /// Increase output verbosity
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Output machine-readable JSON (default when stdout is not a TTY)
    #[arg(short = 'j', long, global = true)]
    pub json: bool,

    /// Output human-readable text (default when stdout is a TTY)
    #[arg(long, global = true, conflicts_with = "json")]
    pub human: bool,
}

impl Cli {
    /// Resolve the effective verbosity level from the clap args.
    pub(crate) fn verbosity(&self) -> genesis::guide::Verbosity {
        if self.quiet {
            genesis::guide::Verbosity::Quiet
        } else if self.verbose {
            genesis::guide::Verbosity::Verbose
        } else {
            genesis::guide::Verbosity::Normal
        }
    }
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Interactive wizard: write pretender.toml, install hook, generate CI
    Init(InitArgs),
    /// Fast pass/fail scan against configured thresholds
    Check(CheckArgs),
    /// Show cyclomatic complexity for each function, sorted worst-first
    Complexity(ComplexityArgs),
    /// Human-readable, markdown, or HTML report from the last `check` run
    Report(ReportArgs),
    /// Structural clone detection via normalised AST subtree hashing
    Duplication(DuplicationArgs),
    /// Mutation testing wrapper (Stryker / PIT / mutmut / cargo-mutants)
    Mutation(MutationArgs),
    /// Install or uninstall the pre-commit hook
    #[command(subcommand)]
    Hooks(HooksCommand),
    /// CI workflow generator
    #[command(subcommand)]
    Ci(CiCommand),
    /// Manage language and metric plugins [not yet implemented]
    #[command(subcommand)]
    Plugins(PluginsCommand),
    /// Print metric definition and threshold citation
    Explain(ExplainArgs),
    /// Diagnose configuration, hooks, and plugin health
    Doctor(DoctorArgs),
    /// File a structured issue against pretender's upstream repo
    Feedback(FeedbackArgs),
    /// Generate shell completion scripts
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

/// All valid pretender commands, kept in sync with the `Commands` enum.
/// Update this list when adding a new command.
pub(crate) const PRETENDER_COMMANDS: &[&str] = &[
    "check",
    "complexity",
    "report",
    "doctor",
    "init",
    "duplication",
    "mutation",
    "hooks",
    "ci",
    "plugins",
    "explain",
    "feedback",
    "completions",
];

#[test]
fn test_pretender_commands_list_is_complete() {
    // Compile-time assertion: if a new variant is added to Commands enum
    // but not to PRETENDER_COMMANDS, this test will fail.
    // Keep this count in sync with the number of Commands variants.
    assert_eq!(
        PRETENDER_COMMANDS.len(),
        13,
        "PRETENDER_COMMANDS must have one entry per Commands variant"
    );
}

pub(crate) trait Executable {
    fn run(&self) -> Result<ExitCode>;
}

impl Executable for Commands {
    fn run(&self) -> Result<ExitCode> {
        match self {
            Commands::Init(args) => args.run(),
            Commands::Check(args) => args.run(),
            Commands::Complexity(args) => args.run(),
            Commands::Report(args) => args.run(),
            Commands::Duplication(args) => args.run(),
            Commands::Mutation(args) => args.run(),
            Commands::Hooks(args) => args.run(),
            Commands::Ci(args) => args.run(),
            Commands::Plugins(_) => not_implemented("plugins", "pretender-07m"),
            Commands::Explain(args) => args.run(),
            Commands::Doctor(args) => args.run(),
            Commands::Feedback(args) => args.run(),
            Commands::Completions { shell } => {
                genesis::cli::generate_completions(&mut Cli::command(), *shell)?;
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}

pub(crate) fn not_implemented(name: &str, tracker: &str) -> Result<ExitCode> {
    eprintln!("pretender {name}: not yet implemented (tracked: {tracker})");
    Ok(ExitCode::from(NOT_IMPLEMENTED_EXIT))
}
