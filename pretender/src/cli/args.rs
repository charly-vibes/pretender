//! CLI argument and subcommand enums.
//!
//! The clap-derive surface of the binary: per-command argument structs,
//! subcommand enums, and the value-enum types shared with other commands.
//!
//! # Responsibilities
//! Declare the parsed CLI vocabulary (flags, defaults, conflicts) exactly
//! as the CLI exposes it; no behavior lives here beyond the two `From`
//! conversions into genesis/prettender domain enums.
//!
//! # Rationale
//! Extracted verbatim from `main.rs` (pretender-5zo.2.5) so the bin root
//! shrinks to dispatch; items are `pub(crate)` so sibling CLI modules and
//! the check/events pipelines can name the shared formats. `CheckArgs`
//! stays at the crate root, flattened from `CheckScanArgs` (what to look
//! at) and `CheckReportArgs` (how to write results) so no struct exceeds
//! the field cap (pretender-5zo.15.7).
use clap::{Args, Parser, Subcommand, ValueEnum};
use genesis::git_hooks::HookName;
use pretender::config::Mode;
use std::path::PathBuf;

/// `check` scan-selection flags: which files the pipeline looks at.
#[derive(Args)]
pub(crate) struct CheckScanArgs {
    /// Paths to files or directories to check for complexity metrics
    #[arg(default_values = ["."])]
    pub(crate) paths: Vec<PathBuf>,
    /// Only check git-staged files (mutually exclusive with --diff-only)
    #[arg(long, conflicts_with = "diff_only")]
    pub(crate) staged: bool,
    /// Only check files changed relative to `diff_base`
    #[arg(long)]
    pub(crate) diff_only: bool,
    /// Override `diff_base` from config
    #[arg(long)]
    pub(crate) diff_base: Option<String>,
    /// Scan git-ignored files too (overrides the default .gitignore-aware
    /// walk; same as setting `pretender.respect_gitignore = false`)
    #[arg(long)]
    pub(crate) scan_ignored: bool,
}

/// `check` report-shaping flags: how results are rendered and written.
#[derive(Args)]
pub(crate) struct CheckReportArgs {
    #[arg(long, value_enum, default_value_t = ReportFormat::Human)]
    pub(crate) format: ReportFormat,
    /// Write report to this path instead of stdout
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
    /// Show all functions with metrics, not just violating ones
    #[arg(long)]
    pub(crate) show_all: bool,
}

#[derive(Parser)]
pub(crate) struct ComplexityArgs {
    /// Paths to files or directories to analyze
    #[arg(default_values = ["."])]
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Parser)]
pub(crate) struct ReportArgs {
    /// Output format
    #[arg(long, value_enum, default_value_t = LongReportFormat::Human)]
    pub(crate) format: LongReportFormat,
    /// Write report to this path instead of stdout
    #[arg(long)]
    pub(crate) output: Option<PathBuf>,
}

#[derive(Parser)]
pub(crate) struct DuplicationArgs {
    /// Paths to scan for clones (default: .)
    #[arg(default_values = ["."])]
    pub(crate) paths: Vec<PathBuf>,
    /// Minimum AST node count to consider a clone (default: 10)
    #[arg(long)]
    pub(crate) min_nodes: Option<u32>,
    /// Detect clones that span multiple files
    #[arg(long)]
    pub(crate) cross_file: bool,
}

#[derive(Parser)]
pub(crate) struct MutationArgs {
    /// Paths to scan for mutation targets (default: .)
    #[arg(default_values = ["."])]
    pub(crate) paths: Vec<PathBuf>,
    /// Minimum mutation score (0–100); exit non-zero if below threshold
    #[arg(long, default_value_t = 60)]
    pub(crate) score_min: u32,
    /// List planned mutants without running tests
    #[arg(long)]
    pub(crate) dry_run: bool,
    #[arg(long, value_enum, default_value_t = ReportFormat::Human)]
    pub(crate) format: ReportFormat,
}

#[derive(Subcommand)]
pub(crate) enum HooksCommand {
    /// Write .git/hooks/<hook> (native shim) — default: pre-commit
    Install {
        /// Which git hook to install
        #[arg(default_value = "pre-commit")]
        hook: HookNameArg,
    },
    /// Remove the hook file(s) previously installed by `pretender hooks install`
    Uninstall {
        /// Which git hook to uninstall
        #[arg(default_value = "pre-commit")]
        hook: HookNameArg,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum HookNameArg {
    PreCommit,
    PrePush,
}

impl From<HookNameArg> for HookName {
    fn from(value: HookNameArg) -> Self {
        match value {
            HookNameArg::PreCommit => HookName::PreCommit,
            HookNameArg::PrePush => HookName::PrePush,
        }
    }
}

#[derive(Subcommand)]
pub(crate) enum CiCommand {
    /// Generate a CI workflow for the given provider
    Generate {
        #[arg(value_enum)]
        provider: CiProvider,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum CiProvider {
    Github,
    Gitlab,
    Circle,
    Azure,
    Generic,
}

#[derive(Subcommand)]
pub(crate) enum PluginsCommand {
    /// Show installed plugins and their versions
    List,
    /// Install a plugin from a git URL or local path
    Add { source: String },
    /// Uninstall a plugin
    Remove { name: String },
}

#[derive(Parser)]
pub(crate) struct ExplainArgs {
    pub(crate) metric: String,
}

#[derive(Parser)]
pub(crate) struct DoctorArgs {}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum ReportFormat {
    Human,
    Json,
    Sarif,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum LongReportFormat {
    Human,
    Markdown,
    Html,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(crate) enum ModeArg {
    Guidance,
    Tiered,
    Gate,
}

impl From<ModeArg> for Mode {
    fn from(value: ModeArg) -> Self {
        match value {
            ModeArg::Guidance => Mode::Guidance,
            ModeArg::Tiered => Mode::Tiered,
            ModeArg::Gate => Mode::Gate,
        }
    }
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum FeedbackKind {
    Bug,
    Feature,
    Question,
}

#[derive(Parser)]
pub(crate) struct FeedbackArgs {
    /// Kind of issue to file
    #[arg(value_enum)]
    pub(crate) kind: FeedbackKind,
    /// Print what would be done without creating the issue
    #[arg(long)]
    pub(crate) dry_run: bool,
    /// Read error context from last error scratch
    #[arg(long)]
    pub(crate) from_last_error: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn duplication_args_default_paths_is_cwd() {
        let args = DuplicationArgs::parse_from(["duplication"]);
        assert_eq!(args.paths, vec![PathBuf::from(".")]);
    }

    #[test]
    fn mutation_args_default_paths_is_cwd() {
        let args = MutationArgs::parse_from(["mutation"]);
        assert_eq!(args.paths, vec![PathBuf::from(".")]);
    }
}
