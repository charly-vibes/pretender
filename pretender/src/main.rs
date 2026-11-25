mod check;
mod report;

use crate::check::collect_input_files;
use crate::check::gate::{violation_finding_id, violation_findings};
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
use std::io::{self, IsTerminal, Read, Write};
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
struct InitArgs {
    /// Skip prompts, use best-guess defaults
    #[arg(long)]
    non_interactive: bool,
    /// Override default mode
    #[arg(long, value_enum)]
    mode: Option<ModeArg>,
    /// Threshold preset to write into the generated config
    #[arg(long, value_enum)]
    preset: Option<ThresholdPreset>,
}

/// Threshold strictness preset for `init` (update-app-threshold-defaults):
/// selects which threshold tables the generated pretender.toml emits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum ThresholdPreset {
    Default,
    Strict,
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

impl Executable for InitArgs {
    fn run(&self) -> Result<ExitCode> {
        let options = if self.non_interactive {
            InitOptions {
                mode: self.mode.unwrap_or(ModeArg::Tiered),
                preset: self.preset.unwrap_or(ThresholdPreset::Default),
                languages: vec!["auto".to_string()],
                install_hook: false,
                generate_github_actions: false,
            }
        } else {
            prompt_init_options(self.mode, self.preset)?
        };

        let cwd = std::env::current_dir().context("failed to get current directory")?;

        // Use genesis::scaffold to write the config and set up directories
        let config_content = render_init_config(&options);
        let result = genesis::scaffold::Scaffold::new(&cwd)
            .default_config("pretender.toml", config_content)
            .build()
            .context("failed to scaffold init")?;

        if result.created.is_empty() && result.existed.iter().any(|p| p.ends_with("pretender.toml"))
        {
            eprintln!("pretender.toml already exists, skipping");
        }

        // Strict on gate is an immediate wall of violations on an existing
        // codebase — point at the migration path (spec: Strict Preset Gate
        // Hint). Printed whenever gate+strict combine, written or not.
        if options.mode == ModeArg::Gate && options.preset == ThresholdPreset::Strict {
            println!(
                "Strict preset with gate mode: existing code will likely violate the tighter thresholds."
            );
            println!(
                "Baseline the current state first: pretender check --adopt, or use mode = \"tiered\" with an advisory lease."
            );
        }

        // Register in genesis discovery manifest
        genesis::discovery::register(
            &cwd,
            "pretender",
            "Multi-language structural code-quality checks",
            "file",
            "pretender.toml",
        )
        .map_err(|e| anyhow::anyhow!("failed to register in genesis manifest: {e}"))?;

        if options.install_hook {
            install_hook(HookName::PreCommit)?;
        }
        if options.generate_github_actions {
            write_github_ci_workflow()?;
        }

        Ok(ExitCode::SUCCESS)
    }
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

struct InitOptions {
    mode: ModeArg,
    preset: ThresholdPreset,
    languages: Vec<String>,
    install_hook: bool,
    generate_github_actions: bool,
}

fn prompt_init_options(
    mode_override: Option<ModeArg>,
    preset_override: Option<ThresholdPreset>,
) -> Result<InitOptions> {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout().lock();

    let mode = match mode_override {
        Some(mode) => mode,
        None => match prompt(
            &mut stdin,
            &mut stdout,
            "Mode (guidance/tiered/gate) [tiered]: ",
        )?
        .trim()
        .to_ascii_lowercase()
        .as_str()
        {
            "" | "tiered" => ModeArg::Tiered,
            "guidance" => ModeArg::Guidance,
            "gate" => ModeArg::Gate,
            other => return Err(anyhow!("invalid mode: {other}")),
        },
    };

    // An explicit --preset suppresses the prompt (mirrors --mode).
    let preset = match preset_override {
        Some(preset) => preset,
        None => match prompt(
            &mut stdin,
            &mut stdout,
            "Threshold preset (default/strict) [default]: ",
        )?
        .trim()
        .to_ascii_lowercase()
        .as_str()
        {
            "" | "default" => ThresholdPreset::Default,
            "strict" => ThresholdPreset::Strict,
            other => return Err(anyhow!("invalid preset: {other}")),
        },
    };

    let languages_input = prompt(
        &mut stdin,
        &mut stdout,
        "Languages (auto or comma-separated list) [auto]: ",
    )?;
    let languages = parse_languages(&languages_input);

    let install_hook = parse_yes_no(&prompt(
        &mut stdin,
        &mut stdout,
        "Install pre-commit hook? [y/N]: ",
    )?);
    let generate_github_actions = parse_yes_no(&prompt(
        &mut stdin,
        &mut stdout,
        "Generate GitHub Actions workflow? [y/N]: ",
    )?);

    Ok(InitOptions {
        mode,
        preset,
        languages,
        install_hook,
        generate_github_actions,
    })
}

fn prompt(stdin: &mut dyn Read, stdout: &mut dyn Write, message: &str) -> Result<String> {
    write!(stdout, "{message}")?;
    stdout.flush()?;

    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        match stdin.read(&mut byte)? {
            0 => break,
            _ if byte[0] == b'\n' => break,
            _ => buf.push(byte[0]),
        }
    }

    Ok(String::from_utf8_lossy(&buf).trim().to_string())
}

fn parse_languages(input: &str) -> Vec<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("auto") {
        return vec!["auto".to_string()];
    }

    let mut languages: Vec<String> = trimmed
        .split(',')
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect();
    if languages.is_empty() {
        languages.push("auto".to_string());
    }
    languages
}

fn parse_yes_no(input: &str) -> bool {
    matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn render_init_config(options: &InitOptions) -> String {
    let mode = match options.mode {
        ModeArg::Guidance => "guidance",
        ModeArg::Tiered => "tiered",
        ModeArg::Gate => "gate",
    };
    let languages = options
        .languages
        .iter()
        .map(|language| format!("\"{language}\""))
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        concat!(
            "[pretender]\n",
            "mode = \"{mode}\"\n",
            "languages = [{languages}]\n",
            "exclude = [\"vendor/**\", \"node_modules/**\", \"**/*_generated.*\"]\n\n",
            "[roles.test]\n",
            "paths = [\"tests/**\", \"**/*_test.*\", \"spec/**\"]\n\n",
            "[roles.library]\n",
            "paths = [\"pkg/**\", \"lib/**\"]\n\n",
            "[roles.script]\n",
            "paths = [\"scripts/**\", \"examples/**\"]\n\n",
            "[roles.generated]\n",
            "paths = [\"**/*.pb.go\", \"**/*_generated.*\"]\n\n",
            "[roles.vendor]\n",
            "paths = [\"vendor/**\", \"node_modules/**\"]\n\n",
            "{thresholds}"
        ),
        mode = mode,
        languages = languages,
        thresholds = preset_tables(options.preset),
    )
}

/// Single source for the threshold tables `init` emits (update-app-threshold-
/// defaults): emitting them explicitly makes the generated config self-
/// documenting and pins today's values against future default changes.
/// Strict deviations are pinned by the spec ("Strict Preset Values"); every
/// other value matches the built-in defaults.
fn preset_tables(preset: ThresholdPreset) -> String {
    let strict = preset == ThresholdPreset::Strict;
    let v = |strict_val, default_val| {
        if strict {
            strict_val
        } else {
            default_val
        }
    };

    format!(
        concat!(
            "[thresholds]\n",
            "cyclomatic_max = {app_cyclomatic}\n",
            "cognitive_max = {app_cognitive}\n",
            "function_lines_max = {app_function_lines}\n",
            "file_lines_max = {app_file_lines}\n",
            "nesting_max = {app_nesting}\n",
            "params_max = {app_params}\n",
            "abc_max = {app_abc}\n",
            "duplication_pct_max = {app_duplication}\n",
            "void_mutators_max = 3\n",
            "mut_ratio_max = 0.3\n",
            "unwrap_max = 5\n",
            "bool_cluster_max = 3\n",
            "primitive_param_check = false\n",
            "inheritance_depth_max = 2\n\n",
            "[thresholds.test]\n",
            "cyclomatic_max = {test_cyclomatic}\n",
            "cognitive_max = {test_cognitive}\n",
            "function_lines_max = {test_function_lines}\n",
            "nesting_max = 2\n",
            "params_max = 2\n",
            "duplication_pct_max = 30\n",
            "min_assertions = 1\n",
            "mock_count_max = 5\n",
            "unwrap_max = 5\n",
            "lazy_cluster_min = 3\n"
        ),
        app_cyclomatic = v(8, 10),
        app_cognitive = v(12, 15),
        app_function_lines = v(25, 30),
        app_file_lines = v(300, 400),
        app_nesting = v(2, 3),
        app_params = v(3, 4),
        app_abc = v(25, 30),
        app_duplication = v(4, 5),
        test_cyclomatic = v(2, 3),
        test_cognitive = v(3, 5),
        test_function_lines = v(60, 80),
    )
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

fn github_ci_workflow_contents() -> &'static str {
    r#"name: Pretender

on:
  pull_request:
  push:
    branches:
      - main

jobs:
  pretender:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      security-events: write
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Install Pretender
        run: cargo install --git https://github.com/charly-vibes/pretender --locked pretender
      - name: Run Pretender
        id: pretender
        continue-on-error: true
        run: pretender check . --format=sarif --output=pretender.sarif
      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: pretender.sarif
      - name: Append markdown report
        if: steps.pretender.outcome == 'failure'
        run: pretender report --format=markdown >> $GITHUB_STEP_SUMMARY
      - name: Fail on Pretender findings
        if: steps.pretender.outcome == 'failure'
        run: exit 1
"#
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

/// Output options for history emission: report format and whether the
/// sink is the process stdout (which enables color).
struct EmitOpts {
    format: ReportFormat,
    stdout: bool,
}

fn emit_history_events(
    report: &mut CheckReport,
    config: &config::Config,
    opts: &EmitOpts,
    sink: &mut dyn Write,
) {
    let timestamp = history::now_iso8601();
    let run_id = history::make_run_id();
    let mode_str = format!("{:?}", config.pretender.mode).to_ascii_lowercase();
    let new_events = all_violation_events(report, &run_id, &timestamp, &mode_str);
    // Snapshot runs on EVERY check (including clean runs) so that "fixed" is
    // distinguishable from "not rescanned" — the core of ticket u8a.
    let current_ids: Vec<String> = new_events.iter().map(|e| e.fingerprint.clone()).collect();
    let store = history::EventStore::new(Path::new(".pretender"));
    let previous = match store.load_last_snapshot() {
        Ok(prev) => prev,
        Err(err) => {
            eprintln!("pretender: history read failed: {err}");
            return;
        }
    };
    let delta = history::compute_delta(
        previous.as_ref().map(|s| s.finding_ids.as_slice()),
        &current_ids,
    );

    // Carry first_seen timestamps forward so persistence duration is tracked
    // (consumed later by dj5's gate-findings export).
    let mut first_seen = previous
        .as_ref()
        .map(|s| s.first_seen.clone())
        .unwrap_or_default();
    for id in &current_ids {
        first_seen
            .entry(id.clone())
            .or_insert_with(|| timestamp.clone());
    }
    let snapshot = history::FindingsSnapshot {
        schema_version: 1,
        run_id: run_id.clone(),
        timestamp: timestamp.clone(),
        finding_ids: current_ids.clone(),
        first_seen: first_seen.clone(),
    };
    if let Err(err) = store.persist_last_snapshot(&snapshot) {
        eprintln!("pretender: history write failed: {err}");
    }

    if new_events.is_empty() {
        if !delta.fixed.is_empty() && matches!(opts.format, ReportFormat::Human) {
            let color = opts.stdout && color_enabled();
            let (green, reset) = if color {
                ("\u{1b}[32m", "\u{1b}[0m")
            } else {
                ("", "")
            };
            let _ = writeln!(
                sink,
                "{green}✓{reset} resolution: {} fixed since last check",
                delta.fixed.len()
            );
        }
        return;
    }
    let all_events = match store.append_and_prune(&new_events) {
        Ok(events) => events,
        Err(err) => {
            eprintln!("pretender: history write failed: {err}");
            return;
        }
    };
    let mut summary = history::compute_summary(&all_events);
    summary.resolution = Some(history::ResolutionStats {
        fixed: delta.fixed.len(),
        still_open: delta.still_open.len(),
        new: delta.new.len(),
        rate: delta.resolution_rate,
    });
    let _ = store.persist_summary(&summary);

    if matches!(opts.format, ReportFormat::Human) {
        let color = opts.stdout && color_enabled();
        let _ = write_resolution_delta(sink, &delta, color);
    }

    // Attach history to the report so it appears in JSON output
    if !summary.top_hotspots.is_empty()
        || !summary.top_patterns.is_empty()
        || summary.resolution.is_some()
    {
        report.history = Some(summary.clone());
        if matches!(opts.format, ReportFormat::Human) {
            let color = opts.stdout && color_enabled();
            let _ = write_recurrence_hints(sink, &summary, color);
        }
    }
}

/// Human-readable delta line: "resolution: 3 fixed, 5 still open, 2 new (43%)"
fn write_resolution_delta(
    sink: &mut dyn Write,
    delta: &history::FindingDelta,
    color: bool,
) -> Result<()> {
    let (cyan, reset) = if color {
        ("\u{1b}[36m", "\u{1b}[0m")
    } else {
        ("", "")
    };
    let rate = delta
        .resolution_rate
        .map(|r| format!(" ({:.0}%)", r * 100.0))
        .unwrap_or_default();
    writeln!(
        sink,
        "{cyan}→{reset} resolution: {} fixed, {} still open, {} new{rate}",
        delta.fixed.len(),
        delta.still_open.len(),
        delta.new.len()
    )?;
    Ok(())
}

fn metric_to_rule_key(metric: &str) -> &str {
    match metric {
        "cognitive" => "cognitive_max",
        "cyclomatic" => "cyclomatic_max",
        "params" => "params_max",
        "nesting" => "nesting_max",
        "function_lines" => "function_lines_max",
        "abc" => "abc_max",
        "duplication_pct" => "duplication_pct_max",
        "min_assertions" => "min_assertions",
        other => other,
    }
}

fn metric_to_family(metric: &str) -> &str {
    match metric {
        "cognitive" | "cyclomatic" | "nesting" | "abc" => "complexity",
        "params" | "function_lines" => "size",
        "duplication_pct" => "duplication",
        "min_assertions" => "testing",
        other => other,
    }
}

fn all_violation_events(
    report: &CheckReport,
    run_id: &str,
    timestamp: &str,
    mode: &str,
) -> Vec<history::ViolationEvent> {
    violation_findings(report)
        .into_iter()
        .map(|finding| {
            let metric_family = metric_to_family(&finding.metric).to_string();
            let scope = if finding.unit_name.is_some() {
                "unit"
            } else {
                "file"
            }
            .to_string();
            let fingerprint = violation_finding_id(&finding);
            history::ViolationEvent {
                schema_version: 1,
                timestamp: timestamp.to_string(),
                run_id: run_id.to_string(),
                mode: mode.to_string(),
                path: finding.path,
                unit_name: finding.unit_name,
                role: finding.role,
                rule_key: finding.rule_key,
                metric_family,
                scope,
                severity: "red".to_string(),
                actual: finding.actual,
                limit: finding.limit,
                delta: finding.actual - finding.limit,
                fingerprint,
            }
        })
        .collect()
}

fn write_recurrence_hints(
    sink: &mut dyn Write,
    summary: &history::HistorySummary,
    color: bool,
) -> Result<()> {
    let cyan = if color { "\u{1b}[36m" } else { "" };
    let bold = if color { "\u{1b}[1m" } else { "" };
    let reset = if color { "\u{1b}[0m" } else { "" };

    writeln!(sink)?;
    writeln!(sink, "{bold}Recurring violations{reset}")?;
    for h in &summary.top_hotspots {
        writeln!(
            sink,
            "  {cyan}hotspot{reset} {} — {} occurrences across {} days",
            h.fingerprint, h.count, h.distinct_days
        )?;
    }
    for p in &summary.top_patterns {
        writeln!(
            sink,
            "  {cyan}pattern{reset} {} in {} ({}) — {} occurrences across {} files",
            p.rule_key, p.area, p.role, p.count, p.distinct_files
        )?;
    }
    Ok(())
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

#[cfg(test)]
mod init_preset_tests {
    use super::*;

    /// Spawn the CLI in `dir` with a neutral GIT_CONFIG_GLOBAL so hooksPath
    /// resolution never sees the developer machine's real global config
    /// (same hermeticity pattern as tests/cli_test.rs pretender_cmd()).
    fn init_cmd(dir: &std::path::Path) -> std::process::Command {
        let bin =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../target/debug/pretender");
        let neutral = std::env::temp_dir().join(format!(
            "pretender-init-preset-test-gitconfig-{}",
            std::process::id()
        ));
        std::fs::write(&neutral, "").expect("write neutral gitconfig");
        let mut cmd = std::process::Command::new(bin);
        cmd.current_dir(dir).env("GIT_CONFIG_GLOBAL", neutral);
        cmd.arg("init");
        cmd
    }

    fn read_config(dir: &std::path::Path) -> String {
        std::fs::read_to_string(dir.join("pretender.toml")).expect("config written")
    }

    /// Contract test
    /// (.espectacular/cli-and-config/interactive-init-asks-for-preset.toml):
    /// the preset choice — prompt answer or --preset flag — selects the
    /// threshold tables init emits; an explicit --preset suppresses the
    /// interactive preset prompt (mirroring --mode).
    #[test]
    fn init_preset_prompt_writes_threshold_tables() {
        // --preset strict --non-interactive: strict tables, app cyclomatic 8
        let dir = tempfile::TempDir::new().expect("tempdir");
        let out = init_cmd(dir.path())
            .args(["--non-interactive", "--preset", "strict"])
            .output()
            .expect("run init");
        assert!(
            out.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let strict_config = read_config(dir.path());
        assert!(
            strict_config.contains("cyclomatic_max = 8"),
            "strict app table missing: {strict_config}"
        );

        // Task 3.3: emitted strict TOML parses back to the pinned strict
        // effective values (spec: Strict Preset Values) via toml::from_str.
        let parsed: Config = toml::from_str(&strict_config).expect("strict config parses");
        assert_eq!(parsed.thresholds.app.cyclomatic_max, 8);
        assert_eq!(parsed.thresholds.app.cognitive_max, 12);
        assert_eq!(parsed.thresholds.app.function_lines_max, 25);
        assert_eq!(parsed.thresholds.app.nesting_max, 2);
        assert_eq!(parsed.thresholds.app.params_max, 3);
        assert_eq!(parsed.thresholds.app.abc_max, 25);
        assert_eq!(parsed.thresholds.app.duplication_pct_max, 4);
        assert_eq!(parsed.thresholds.app.file_lines_max, 300);
        assert_eq!(parsed.thresholds.test.cyclomatic_max, 2);
        assert_eq!(parsed.thresholds.test.cognitive_max, 3);
        assert_eq!(parsed.thresholds.test.function_lines_max, 60);
        assert_eq!(parsed.thresholds.test.nesting_max, 2);
        // enabled-check values identical to the default preset
        assert_eq!(parsed.thresholds.app.unwrap_max, 5);
        assert_eq!(parsed.thresholds.test.mock_count_max, 5);

        // --preset default writes the default tables
        let dir = tempfile::TempDir::new().expect("tempdir");
        let out = init_cmd(dir.path())
            .args(["--non-interactive", "--preset", "default"])
            .output()
            .expect("run init");
        assert!(
            out.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let default_config = read_config(dir.path());
        assert!(
            default_config.contains("cyclomatic_max = 10"),
            "default app table missing: {default_config}"
        );
        let parsed: Config = toml::from_str(&default_config).expect("default config parses");
        assert_eq!(
            parsed.thresholds.app,
            Config::default().thresholds.app,
            "default preset tables must round-trip to the built-in app defaults"
        );
        assert_eq!(
            parsed.thresholds.test,
            Config::default().thresholds.test,
            "default preset tables must round-trip to the built-in test defaults"
        );

        // Absent flag: same as --preset default
        let dir = tempfile::TempDir::new().expect("tempdir");
        let out = init_cmd(dir.path())
            .arg("--non-interactive")
            .output()
            .expect("run init");
        assert!(
            out.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            read_config(dir.path()).contains("cyclomatic_max = 10"),
            "absent --preset must emit the default tables"
        );

        // Explicit --preset on an interactive run suppresses the preset
        // prompt: stdin answers mode/languages/hook/workflow only; had the
        // prompt not been suppressed, the answers would shift and the run
        // would fail (or fall back to default tables).
        let dir = tempfile::TempDir::new().expect("tempdir");
        let mut child = init_cmd(dir.path())
            .arg("--preset")
            .arg("strict")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("spawn interactive init");
        {
            use std::io::Write;
            let stdin = child.stdin.as_mut().expect("piped stdin");
            stdin
                .write_all(b"\nauto\nn\nn\n")
                .expect("write prompt answers");
        }
        let out = child.wait_with_output().expect("interactive init finishes");
        assert!(
            out.status.success(),
            "interactive init with explicit --preset failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            read_config(dir.path()).contains("cyclomatic_max = 8"),
            "explicit --preset must suppress the preset prompt and emit strict tables"
        );
    }

    #[test]
    fn init_rejects_unknown_preset() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let out = init_cmd(dir.path())
            .args(["--non-interactive", "--preset", "loose"])
            .output()
            .expect("run init");
        assert!(!out.status.success(), "unknown preset must exit non-zero");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("loose"),
            "error should name the bad value: {err}"
        );
    }

    #[test]
    fn init_strict_gate_prints_migration_hint() {
        // Spec: Strict Preset Gate Hint — gate + strict must print the
        // --adopt / tiered-lease migration hint after writing the config.
        let dir = tempfile::TempDir::new().expect("tempdir");
        let out = init_cmd(dir.path())
            .args(["--non-interactive", "--mode", "gate", "--preset", "strict"])
            .output()
            .expect("run init");
        assert!(
            out.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("check --adopt"),
            "gate+strict must print the adoption hint; stdout: {stdout}"
        );
    }

    #[test]
    fn init_default_gate_no_migration_hint() {
        // The hint is scoped to gate+strict; default preset stays quiet.
        let dir = tempfile::TempDir::new().expect("tempdir");
        let out = init_cmd(dir.path())
            .args(["--non-interactive", "--mode", "gate"])
            .output()
            .expect("run init");
        assert!(
            out.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains("check --adopt"),
            "gate+default must not print the adoption hint; stdout: {stdout}"
        );
    }
}
