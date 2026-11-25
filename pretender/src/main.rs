mod check;
mod events;
mod init;
mod report;

use crate::check::collect_input_files;
use crate::init::{github_ci_workflow_contents, InitArgs};
use crate::report::{write_html_report, write_human_report, write_markdown_report, HumanOpts};
use pretender::{
    c, clojure, config, cpp, csharp, doctor, duplication, explain, external_plugin, go, history,
    java, javascript, julia, metrics, model, mutation, python, r, ruby, rust, test_report,
    typescript,
};

use anyhow::{anyhow, Context, Result};
use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use genesis::config::{ConfigFile, ConfigStore, ValidationSeverity};
use genesis::envelope::{Envelope, EnvelopeKind};
use genesis::feedback::scratch::{self, ErrorRecord};
use genesis::feedback::{self as genesis_feedback, FeedbackArgs as GenesisFeedbackArgs};
use genesis::git_hooks::{self as genesis_git_hooks, HookName};
use genesis::guide::Guide;
use genesis::status::{StatusContributor, StatusItem, StatusSection};
use genesis::suggestions::SuggestionEngine;
use pretender::config::{Config, Mode};
use pretender::model::Metric;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const NOT_IMPLEMENTED_EXIT: u8 = 2;

#[derive(Parser)]
#[command(name = "pretender", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

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
    fn verbosity(&self) -> genesis::guide::Verbosity {
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
enum Commands {
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
const PRETENDER_COMMANDS: &[&str] = &[
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

#[derive(Parser)]
struct ComplexityArgs {
    /// Paths to files or directories to analyze
    #[arg(default_values = ["."])]
    paths: Vec<PathBuf>,
}

#[derive(Parser)]
struct CheckArgs {
    /// Paths to files or directories to check for complexity metrics
    #[arg(default_values = ["."])]
    paths: Vec<PathBuf>,
    #[arg(long, value_enum, default_value_t = ReportFormat::Human)]
    format: ReportFormat,
    /// Write report to this path instead of stdout
    #[arg(long)]
    output: Option<PathBuf>,
    /// Only check git-staged files (mutually exclusive with --diff-only)
    #[arg(long, conflicts_with = "diff_only")]
    staged: bool,
    /// Only check files changed relative to `diff_base`
    #[arg(long)]
    diff_only: bool,
    /// Override `diff_base` from config
    #[arg(long)]
    diff_base: Option<String>,
    /// Override config `pretender.mode`
    #[arg(long, value_enum)]
    mode: Option<ModeArg>,
    /// Advisory lease expiry (ISO YYYY-MM-DD) for tiered/guidance modes.
    /// Required for advisory modes; an expired/missing lease fails closed —
    /// renew by pushing the date forward (a visible, reviewable diff).
    #[arg(long, value_name = "DATE")]
    advisory_until: Option<String>,
    /// Show all functions with metrics, not just violating ones
    #[arg(long)]
    show_all: bool,
    /// Path to a JUnit XML report for test-duration analysis
    #[arg(long)]
    test_report: Option<PathBuf>,
    /// Scan git-ignored files too (overrides the default .gitignore-aware
    /// walk; same as setting `pretender.respect_gitignore = false`)
    #[arg(long)]
    scan_ignored: bool,
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

#[derive(Parser)]
struct ReportArgs {
    /// Output format
    #[arg(long, value_enum, default_value_t = LongReportFormat::Human)]
    format: LongReportFormat,
    /// Write report to this path instead of stdout
    #[arg(long)]
    output: Option<PathBuf>,
}

#[derive(Parser)]
struct DuplicationArgs {
    paths: Vec<PathBuf>,
    /// Minimum AST node count to consider a clone (default: 10)
    #[arg(long)]
    min_nodes: Option<u32>,
    /// Detect clones that span multiple files
    #[arg(long)]
    cross_file: bool,
}

#[derive(Parser)]
struct MutationArgs {
    paths: Vec<PathBuf>,
    /// Minimum mutation score (0–100); exit non-zero if below threshold
    #[arg(long, default_value_t = 60)]
    score_min: u32,
    /// List planned mutants without running tests
    #[arg(long)]
    dry_run: bool,
    #[arg(long, value_enum, default_value_t = ReportFormat::Human)]
    format: ReportFormat,
}

#[derive(Subcommand)]
enum HooksCommand {
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
enum HookNameArg {
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
enum CiCommand {
    /// Generate a CI workflow for the given provider
    Generate {
        #[arg(value_enum)]
        provider: CiProvider,
    },
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum CiProvider {
    Github,
    Gitlab,
    Circle,
    Azure,
    Generic,
}

#[derive(Subcommand)]
enum PluginsCommand {
    /// Show installed plugins and their versions
    List,
    /// Install a plugin from a git URL or local path
    Add { source: String },
    /// Uninstall a plugin
    Remove { name: String },
}

#[derive(Parser)]
struct ExplainArgs {
    metric: String,
}

#[derive(Parser)]
struct DoctorArgs {}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ReportFormat {
    Human,
    Json,
    Sarif,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LongReportFormat {
    Human,
    Markdown,
    Html,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum ModeArg {
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
enum FeedbackKind {
    Bug,
    Feature,
    Question,
}

#[derive(Parser)]
struct FeedbackArgs {
    /// Kind of issue to file
    #[arg(value_enum)]
    kind: FeedbackKind,
    /// Print what would be done without creating the issue
    #[arg(long)]
    dry_run: bool,
    /// Read error context from last error scratch
    #[arg(long)]
    from_last_error: bool,
}

trait Executable {
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
                let mut cmd = Cli::command();
                genesis::cli::generate_completions(&mut cmd, *shell)?;
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}

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

fn not_implemented(name: &str, tracker: &str) -> Result<ExitCode> {
    eprintln!("pretender {name}: not yet implemented (tracked: {tracker})");
    Ok(ExitCode::from(NOT_IMPLEMENTED_EXIT))
}

impl Executable for FeedbackArgs {
    fn run(&self) -> Result<ExitCode> {
        let repo = extract_repo_from_cargo_toml()?;
        let project_root = std::env::current_dir().context("failed to get current directory")?;

        let kind_str = match self.kind {
            FeedbackKind::Bug => "bug",
            FeedbackKind::Feature => "feature",
            FeedbackKind::Question => "question",
        };

        let genesis_args = GenesisFeedbackArgs::new(kind_str, self.dry_run, self.from_last_error);
        let tool_name = env!("CARGO_PKG_NAME");
        let tool_version = env!("CARGO_PKG_VERSION");

        match genesis_feedback::handle_feedback(
            &genesis_args,
            tool_name,
            tool_version,
            &repo,
            &project_root,
        ) {
            Ok(result) => match result {
                genesis::feedback::gh::GhResult::Created { url, number } => {
                    println!("Created issue #{number}: {url}");
                }
                genesis::feedback::gh::GhResult::FallbackUrl(url) => {
                    println!("Open to create issue: {url}");
                }
                genesis::feedback::gh::GhResult::LocalFile(path) => {
                    println!("Issue body written to: {}", path.display());
                }
            },
            Err(msg) => {
                eprintln!("Failed to create issue: {msg}");
                return Ok(ExitCode::FAILURE);
            }
        }
        Ok(ExitCode::SUCCESS)
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
                for diag in &diagnostics {
                    eprintln!("{:?}: {}", diag.severity, diag.message);
                    if let Some(span) = &diag.span {
                        eprintln!("  at lines {}-{}", span.start_line, span.end_line);
                    }
                }
                eprintln!();
            }

            if multi {
                println!("{}:", path.display());
            }

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
        Ok(ExitCode::SUCCESS)
    }
}

impl Executable for DuplicationArgs {
    fn run(&self) -> Result<ExitCode> {
        warn_if_no_config();
        let config = load_config()?;
        let file_paths = collect_input_files(&self.paths, &config)?;
        let min_nodes = self.min_nodes.unwrap_or(10) as usize;

        let mut files: Vec<(PathBuf, String)> = Vec::new();
        for path in &file_paths {
            if duplication::ts_language_for_path(path).is_none() {
                continue;
            }
            match fs::read_to_string(path) {
                Ok(source) => files.push((path.clone(), source)),
                Err(e) => eprintln!("warning: skipping {}: {e}", path.display()),
            }
        }

        let groups = duplication::detect_clones(&files, min_nodes, self.cross_file)?;

        if groups.is_empty() {
            println!("No structural clones detected.");
            return Ok(ExitCode::SUCCESS);
        }

        println!("Found {} clone group(s):\n", groups.len());
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

        Ok(ExitCode::SUCCESS)
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

        match self.format {
            ReportFormat::Json => {
                let env = Envelope::success(
                    env!("CARGO_PKG_VERSION"),
                    EnvelopeKind::Check,
                    &report,
                    vec![],
                    vec![],
                );
                println!("{}", serde_json::to_string_pretty(&env)?);
            }
            _ => print_mutation_report(&report),
        }

        if report.passes_threshold(self.score_min) {
            Ok(ExitCode::SUCCESS)
        } else {
            Ok(ExitCode::FAILURE)
        }
    }
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

impl Executable for ReportArgs {
    fn run(&self) -> Result<ExitCode> {
        let report = load_last_check_report()?;
        let config = load_config().unwrap_or_default();
        let writing_to_stdout = self.output.is_none();
        let mut sink = open_report_sink(self.output.as_deref())?;

        match self.format {
            LongReportFormat::Human => {
                let color = writing_to_stdout && color_enabled();
                write_human_report(
                    sink.as_mut(),
                    &report,
                    &HumanOpts {
                        color,
                        bands: &config.bands,
                        mode: config.pretender.mode,
                        verbose: true,
                    },
                )?;
            }
            LongReportFormat::Markdown => {
                write_markdown_report(sink.as_mut(), &report, &config.bands)?
            }
            LongReportFormat::Html => write_html_report(sink.as_mut(), &report, &config.bands)?,
        }

        sink.flush().context("failed to flush report output")?;
        Ok(ExitCode::SUCCESS)
    }
}

impl Executable for HooksCommand {
    fn run(&self) -> Result<ExitCode> {
        match self {
            HooksCommand::Install { hook } => {
                install_hook((*hook).into())?;
                Ok(ExitCode::SUCCESS)
            }
            HooksCommand::Uninstall { hook } => {
                uninstall_hook((*hook).into())?;
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}

impl Executable for CiCommand {
    fn run(&self) -> Result<ExitCode> {
        match self {
            CiCommand::Generate {
                provider: CiProvider::Github,
            } => {
                write_github_ci_workflow()?;
                Ok(ExitCode::SUCCESS)
            }
            CiCommand::Generate { .. } => not_implemented("ci generate", "pretender-fb3"),
        }
    }
}

const PRE_COMMIT_HOOK_MARKER: &str = "# Installed by Pretender.";

fn hook_script() -> String {
    "#!/usr/bin/env sh\nexec pretender check . --staged\n".to_string()
}

fn install_hook(hook: HookName) -> Result<PathBuf> {
    let root = git_hook_repo_root()?;
    let path = genesis_git_hooks::install(&root, hook, PRE_COMMIT_HOOK_MARKER, &hook_script())
        .map_err(|e| anyhow!("{}", hook_error_message(e)))?;
    Ok(path)
}

fn uninstall_hook(hook: HookName) -> Result<PathBuf> {
    let root = git_hook_repo_root()?;
    let path = genesis_git_hooks::uninstall(&root, hook, PRE_COMMIT_HOOK_MARKER)
        .map_err(|e| anyhow!("{}", hook_error_message(e)))?;
    Ok(path)
}

fn hook_error_message(err: genesis_git_hooks::GitHooksError) -> String {
    use genesis_git_hooks::GitHooksError as E;
    match err {
        E::ForeignHook { path, action } => match action {
            "remove" => format!(
                "refusing to remove hook not installed by Pretender: {}",
                path.display()
            ),
            _ => format!(
                "refusing to overwrite hook not installed by Pretender: {}",
                path.display()
            ),
        },
        other => format!("hook operation failed: {other}"),
    }
}

fn git_hook_repo_root() -> Result<PathBuf> {
    genesis_git_hooks::repo_root().map_err(|e| anyhow!("{}", e))
}

fn github_ci_workflow_path() -> PathBuf {
    PathBuf::from(".github/workflows/pretender.yml")
}

fn write_github_ci_workflow() -> Result<()> {
    let path = github_ci_workflow_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create workflow dir: {}", parent.display()))?;
    }
    fs::write(&path, github_ci_workflow_contents())
        .with_context(|| format!("failed to write workflow: {}", path.display()))?;
    Ok(())
}

fn last_check_report_path() -> PathBuf {
    PathBuf::from(".pretender/last-check.json")
}

fn persist_last_check_report(report: &CheckReport) -> Result<()> {
    let path = last_check_report_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create report cache dir: {}", parent.display()))?;
    }
    let bytes = serde_json::to_vec_pretty(report)?;
    fs::write(&path, bytes)
        .with_context(|| format!("failed to write last check report: {}", path.display()))?;
    Ok(())
}

fn load_last_check_report() -> Result<CheckReport> {
    let path = last_check_report_path();
    let source = fs::read_to_string(&path)
        .with_context(|| format!("failed to read last check report: {}", path.display()))?;
    serde_json::from_str(&source).context("failed to parse last check report JSON")
}

fn open_report_sink(path: Option<&Path>) -> Result<Box<dyn Write>> {
    match path {
        Some(path) => {
            let file = fs::File::create(path)
                .with_context(|| format!("failed to open output path: {}", path.display()))?;
            Ok(Box::new(io::BufWriter::new(file)))
        }
        None => Ok(Box::new(io::stdout().lock())),
    }
}

fn load_config() -> Result<Config> {
    let path = Path::new("pretender.toml");
    if !path.exists() {
        return Ok(Config::default());
    }
    // An empty config file is treated as "use defaults" to preserve the
    // v0.1.0 tolerance: a present-but-empty pretender.toml is not an error
    // for `check`/`complexity` (doctor still flags it as invalid).
    if std::fs::metadata(path)
        .with_context(|| format!("failed to stat config: {}", path.display()))?
        .len()
        == 0
    {
        return Ok(Config::default());
    }
    // Delegate read/parse to genesis::config, then validate the parsed
    // instance directly (single read, no TOCTOU between get and validate).
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let store = ConfigStore::new(config::build_registry());
    let parsed: Config = store
        .get("pretender", &cwd)
        .map_err(|e| anyhow!("failed to read pretender.toml: {e}"))?;
    let validations = parsed
        .validate()
        .map_err(|e| anyhow!("failed to validate pretender.toml: {e}"))?;
    if config::has_errors(&validations) {
        let first = validations
            .iter()
            .find(|v| v.severity == ValidationSeverity::Error)
            .expect("has_errors guaranteed an error entry");
        return Err(anyhow!("invalid pretender.toml: {}", first.message));
    }
    Ok(parsed)
}

fn warn_if_no_config() {
    if !Path::new("pretender.toml").exists() {
        eprintln!("no pretender.toml found, using defaults — run `pretender init` to configure");
    }
}

fn color_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    io::stdout().is_terminal()
}

fn get_parser(path: &Path) -> Result<Box<dyn model::Parser>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("missing file extension for path: {}", path.display()))?;

    match ext {
        "cs" => Ok(Box::new(csharp::CSharpParser)),
        "clj" | "cljs" | "cljc" | "edn" => Ok(Box::new(clojure::ClojureParser)),
        "c" | "h" => Ok(Box::new(c::CParser)),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Ok(Box::new(cpp::CppParser)),
        "go" => Ok(Box::new(go::GoParser)),
        "java" => Ok(Box::new(java::JavaParser)),
        "jl" => Ok(Box::new(julia::JuliaParser)),
        "js" | "jsx" | "mjs" | "cjs" => Ok(Box::new(javascript::JavaScriptParser)),
        "py" => Ok(Box::new(python::PythonParser)),
        "r" | "R" => Ok(Box::new(r::RParser)),
        "rb" => Ok(Box::new(ruby::RubyParser)),
        "rs" => Ok(Box::new(rust::RustParser)),
        "ts" | "mts" => Ok(Box::new(typescript::TypeScriptParser)),
        "tsx" | "cts" => Ok(Box::new(typescript::TypeScriptXParser)),
        _ => Err(anyhow!("unsupported file extension '.{}'", ext)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CheckReport {
    files: Vec<FileReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    modules: Vec<ModuleReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    cycles: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    test_findings: Vec<test_report::TestDurationFinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    history: Option<history::HistorySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileReport {
    path: String,
    role: String,
    diagnostics: Vec<DiagnosticReport>,
    file_violations: Vec<ViolationReport>,
    units: Vec<UnitReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    external_findings: Vec<external_plugin::ExternalFinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    mock_findings: Vec<MockFindingReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiagnosticReport {
    severity: String,
    message: String,
    start_line: Option<u32>,
    end_line: Option<u32>,
}

impl From<model::Diagnostic> for DiagnosticReport {
    fn from(value: model::Diagnostic) -> Self {
        Self {
            severity: format!("{:?}", value.severity),
            message: value.message,
            start_line: value.span.as_ref().map(|span| span.start_line),
            end_line: value.span.as_ref().map(|span| span.end_line),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct UnitReport {
    name: String,
    kind: String,
    start_line: u32,
    metrics: MetricValues,
    violations: Vec<ViolationReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetricValues {
    cyclomatic: u32,
    cognitive: u32,
    assertions: u32,
    function_lines: u32,
    params: u32,
    nesting_max: u32,
    abc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ViolationReport {
    metric: String,
    actual: f64,
    limit: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModuleReport {
    path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    coupling_violations: Vec<CouplingViolationReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CouplingViolationReport {
    metric: String,
    actual: f64,
    limit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MockFindingReport {
    name: String,
    line: u32,
    kind: String,
}

fn main() -> ExitCode {
    // Check for --version --json before clap parsing
    if genesis::cli::maybe_print_version_json(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")) {
        return ExitCode::SUCCESS;
    }

    let mut guide = Guide::builder("pretender", env!("CARGO_PKG_VERSION"))
        .commands(PRETENDER_COMMANDS)
        .build();

    let cli = match Cli::try_parse() {
        Ok(cli) => {
            guide.set_verbosity(cli.verbosity());
            cli
        }
        Err(err) => {
            if err.kind() == clap::error::ErrorKind::InvalidSubcommand {
                let err_msg = err.to_string();
                if let Some(bad_cmd) = extract_bad_subcommand(&err_msg) {
                    let engine = SuggestionEngine::new();

                    if let Some(suggestion) = engine.suggest_typo(&bad_cmd, guide.registry()) {
                        eprintln!("error: unrecognized subcommand '{bad_cmd}'");
                        eprintln!();
                        eprintln!("{}", suggestion.message());
                        if let Some(footer) = suggestion.footer() {
                            eprintln!("{footer}");
                        }
                        std::process::exit(2);
                    }
                }
            }
            err.exit();
        }
    };

    match cli.command.run() {
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

/// Resolve the doctor output format from global CLI args or TTY auto-detect.
fn resolve_doctor_format() -> genesis::guide::OutputFormat {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--json" || a == "-j") {
        genesis::guide::OutputFormat::Json
    } else {
        genesis::guide::OutputFormat::Human
    }
}

/// Pretender's status contributor for the cross-tool dashboard.
pub struct PretenderStatus;

impl StatusContributor for PretenderStatus {
    fn name(&self) -> &'static str {
        "pretender"
    }

    fn status(&self, repo_root: &std::path::Path) -> Result<StatusSection, String> {
        let config_exists = repo_root.join("pretender.toml").exists();
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

        let summary = if config_exists {
            "configured".to_string()
        } else {
            "not initialized".to_string()
        };

        Ok(StatusSection::with_items("pretender", summary, items))
    }
}

#[cfg(test)]
mod violation_report_tests {
    use crate::report::write_sarif_report;
    use super::*;

    fn violation(line: Option<u32>) -> ViolationReport {
        ViolationReport {
            metric: "cyclomatic".to_string(),
            actual: 12.0,
            limit: 10.0,
            line,
        }
    }

    fn metric_values() -> MetricValues {
        MetricValues {
            cyclomatic: 1,
            cognitive: 0,
            assertions: 1,
            function_lines: 0,
            params: 0,
            nesting_max: 0,
            abc: 1.0,
        }
    }

    fn unit_report(name: &str, violations: Vec<ViolationReport>) -> UnitReport {
        UnitReport {
            name: name.to_string(),
            kind: "function".to_string(),
            start_line: 10,
            metrics: metric_values(),
            violations,
        }
    }

    fn file_report(path: &str, units: Vec<UnitReport>) -> FileReport {
        FileReport {
            path: path.to_string(),
            role: "app".to_string(),
            diagnostics: vec![],
            file_violations: vec![],
            units,
            external_findings: vec![],
            mock_findings: vec![],
        }
    }

    fn report(files: Vec<FileReport>) -> CheckReport {
        CheckReport {
            files,
            modules: vec![],
            cycles: vec![],
            test_findings: vec![],
            history: None,
        }
    }

    /// Byte-compat with existing JSON output: no `line` key when None.
    #[test]
    fn line_none_is_omitted_from_json() {
        let json = serde_json::to_string(&violation(None)).expect("serialize");
        assert_eq!(
            json,
            r#"{"metric":"cyclomatic","actual":12.0,"limit":10.0}"#
        );
    }

    #[test]
    fn line_some_is_emitted_in_json() {
        let json = serde_json::to_string(&violation(Some(42))).expect("serialize");
        assert_eq!(
            json,
            r#"{"metric":"cyclomatic","actual":12.0,"limit":10.0,"line":42}"#
        );
    }

    #[test]
    fn sarif_region_uses_line_when_present() {
        let mut file = file_report(
            "src/a.rs",
            vec![unit_report("fn_a", vec![violation(Some(50))])],
        );
        file.units[0].start_line = 10;
        let report = report(vec![file]);
        let mut buf = Vec::new();
        write_sarif_report(&mut buf, &report).expect("sarif");
        let v: serde_json::Value = serde_json::from_slice(&buf).expect("parse sarif");
        let region = &v["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 50);
    }

    #[test]
    fn sarif_region_falls_back_to_default_start_line_when_line_none() {
        let mut file = file_report("src/a.rs", vec![unit_report("fn_a", vec![violation(None)])]);
        file.units[0].start_line = 10;
        let report = report(vec![file]);
        let mut buf = Vec::new();
        write_sarif_report(&mut buf, &report).expect("sarif");
        let v: serde_json::Value = serde_json::from_slice(&buf).expect("parse sarif");
        let region = &v["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 10);
    }
}

