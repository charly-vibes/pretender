mod c;
mod config;
mod cpp;
mod duplication;
mod engine;
mod explain;
mod external_plugin;
mod git;
mod go;
mod history;
mod java;
mod javascript;
mod metrics;
mod model;
mod mutation;
mod plugin;
mod python;
mod roles;
mod ruby;
mod rust;
mod typescript;

use crate::config::{Band, Bands, Config, Mode};
use crate::model::Metric;
use crate::roles::{EffectiveThresholds, Role, RoleDetector};
use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Reverse;
use std::collections::HashSet;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const NOT_IMPLEMENTED_EXIT: u8 = 2;

#[derive(Parser)]
#[command(name = "pretender")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Interactive wizard: write pretender.toml, install hook, generate CI
    Init(InitArgs),
    /// Fast pass/fail scan against configured thresholds
    Check(CheckArgs),
    /// Show cyclomatic complexity for each function, sorted worst-first
    Complexity(ComplexityArgs),
    /// Pretty TUI or HTML report from the last `check` run
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
    /// Manage language and metric plugins
    #[command(subcommand)]
    Plugins(PluginsCommand),
    /// Print metric definition and threshold citation
    Explain(ExplainArgs),
}

#[derive(Parser)]
struct InitArgs {
    /// Skip prompts, use best-guess defaults
    #[arg(long)]
    non_interactive: bool,
    /// Override default mode
    #[arg(long, value_enum)]
    mode: Option<ModeArg>,
}

#[derive(Parser)]
struct ComplexityArgs {
    path: PathBuf,
}

#[derive(Parser)]
struct CheckArgs {
    #[arg(required = true)]
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
    #[arg(long)]
    min_nodes: Option<u32>,
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
    /// Write .git/hooks/pre-commit (native shim) or lefthook/pre-commit YAML
    Install,
    /// Remove the hook file(s) previously installed by `pretender hooks install`
    Uninstall,
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

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ReportFormat {
    Human,
    Json,
    Sarif,
    Junit,
    Markdown,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum LongReportFormat {
    Human,
    Markdown,
    Html,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
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
        }
    }
}

impl Executable for ExplainArgs {
    fn run(&self) -> Result<ExitCode> {
        explain::run(&self.metric)?;
        Ok(ExitCode::SUCCESS)
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
                languages: vec!["auto".to_string()],
                install_hook: false,
                generate_github_actions: false,
            }
        } else {
            prompt_init_options(self.mode)?
        };

        fs::write("pretender.toml", render_init_config(&options))
            .context("failed to write pretender.toml")?;

        if options.install_hook {
            install_pre_commit_hook()?;
        }
        if options.generate_github_actions {
            write_github_ci_workflow()?;
        }

        Ok(ExitCode::SUCCESS)
    }
}

impl Executable for ComplexityArgs {
    fn run(&self) -> Result<ExitCode> {
        let source = std::fs::read_to_string(&self.path)
            .with_context(|| format!("failed to read source file: {}", self.path.display()))?;
        let parser = get_parser(&self.path)?;
        let (module, diagnostics) = parser.parse(&self.path, &source)?;

        if !diagnostics.is_empty() {
            for diag in &diagnostics {
                eprintln!("{:?}: {}", diag.severity, diag.message);
                if let Some(span) = &diag.span {
                    eprintln!("  at lines {}-{}", span.start_line, span.end_line);
                }
            }
            eprintln!();
        }

        let metric = metrics::CyclomaticComplexity;
        let mut results: Vec<(String, u32)> = module
            .units
            .iter()
            .map(|u| (u.name.clone(), metric.calculate(u)))
            .collect();

        results.sort_by_key(|(_, score)| Reverse(*score));
        for (name, score) in &results {
            println!("{name}: {score}");
        }
        Ok(ExitCode::SUCCESS)
    }
}

impl Executable for CheckArgs {
    fn run(&self) -> Result<ExitCode> {
        if matches!(self.format, ReportFormat::Junit | ReportFormat::Markdown) {
            return not_implemented(
                &format!("check --format {:?}", self.format).to_lowercase(),
                "pretender-t2m",
            );
        }

        warn_if_no_config();
        let mut config = load_config()?;
        if let Some(mode) = self.mode {
            config.pretender.mode = mode.into();
        }

        let detector = RoleDetector::new(&config).context("failed to initialize role detector")?;

        let files = if self.staged || self.diff_only || self.diff_base.is_some() {
            let cwd = std::env::current_dir().context("failed to get current directory")?;
            let allowed = if self.staged {
                git::staged_files(&cwd)?
            } else {
                let base = self.diff_base.as_deref().unwrap_or(&config.scope.diff_base);
                git::diff_base_files(&cwd, base)?
            };
            // Short-circuit: skip the full directory walk when nothing is staged/changed.
            if allowed.is_empty() {
                return Ok(decide_exit_code(
                    &CheckReport { files: vec![] },
                    config.pretender.mode,
                ));
            }
            let all = collect_input_files(&self.paths, &config)?;
            apply_file_filter(all, Some(&allowed), &cwd)
        } else {
            collect_input_files(&self.paths, &config)?
        };

        let mut reports: Vec<FileReport> = files
            .par_iter()
            .filter_map(|path| analyze_path(path, &detector, &config).transpose())
            .collect::<Result<_>>()?;
        reports.sort_by(|a, b| a.path.cmp(&b.path));

        let plugins = external_plugin::load_plugins(&external_plugin::default_metrics_dir());
        if !plugins.is_empty() {
            let mut external = external_plugin::run_plugins(&plugins, &files);
            for file_report in &mut reports {
                if let Some(findings) = external.remove(&file_report.path) {
                    file_report.external_findings = findings;
                }
            }
        }

        let report = CheckReport { files: reports };
        let writing_to_stdout = self.output.is_none();
        let mut sink = open_report_sink(self.output.as_deref())?;
        match self.format {
            ReportFormat::Human => {
                let color = writing_to_stdout && color_enabled();
                write_human_report(sink.as_mut(), &report, color, &config.bands, config.pretender.mode)?;
            }
            ReportFormat::Json => write_json_report(sink.as_mut(), &report)?,
            ReportFormat::Sarif => write_sarif_report(sink.as_mut(), &report)?,
            _ => unreachable!("junit/markdown handled above"),
        }
        persist_last_check_report(&report)?;
        emit_history_events(
            &report,
            &config,
            self.format,
            writing_to_stdout,
            sink.as_mut(),
        );
        sink.flush().context("failed to flush report output")?;

        Ok(decide_exit_code(&report, config.pretender.mode))
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
        let lang = mutation::primary_lang(&self.paths)
            .ok_or_else(|| anyhow!("no supported source files found in provided paths"))?;

        if self.dry_run {
            return run_mutation_dry_run(&lang, &self.paths);
        }

        let report = mutation::run_mutation(&lang, &self.paths)?;

        match self.format {
            ReportFormat::Json => {
                println!("{}", serde_json::to_string_pretty(&report)?);
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
                write_human_report(sink.as_mut(), &report, color, &config.bands, config.pretender.mode)?;
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
            HooksCommand::Install => {
                install_pre_commit_hook()?;
                Ok(ExitCode::SUCCESS)
            }
            HooksCommand::Uninstall => {
                uninstall_pre_commit_hook()?;
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

fn decide_exit_code(report: &CheckReport, mode: Mode) -> ExitCode {
    let has_skipped = report
        .files
        .iter()
        .any(|file| file.diagnostics.iter().any(|d| d.severity == "Error"));
    let has_violation = report.files.iter().any(|file| {
        !file.file_violations.is_empty()
            || file.units.iter().any(|unit| !unit.violations.is_empty())
    });

    match mode {
        Mode::Guidance => ExitCode::SUCCESS,
        Mode::Tiered => ExitCode::SUCCESS,
        Mode::Gate => {
            if has_violation || has_skipped {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
    }
}

struct InitOptions {
    mode: ModeArg,
    languages: Vec<String>,
    install_hook: bool,
    generate_github_actions: bool,
}

fn prompt_init_options(mode_override: Option<ModeArg>) -> Result<InitOptions> {
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
            "paths = [\"vendor/**\", \"node_modules/**\"]\n"
        ),
        mode = mode,
        languages = languages,
    )
}

const PRE_COMMIT_HOOK_MARKER: &str = "# Installed by Pretender.";

fn install_pre_commit_hook() -> Result<()> {
    let path = PathBuf::from(".git/hooks/pre-commit");
    if path.exists() {
        let existing = fs::read_to_string(&path)
            .with_context(|| format!("failed to read hook: {}", path.display()))?;
        if !existing.contains(PRE_COMMIT_HOOK_MARKER) {
            return Err(anyhow!(
                "refusing to overwrite hook not installed by Pretender: {}",
                path.display()
            ));
        }
    }
    let parent = path
        .parent()
        .ok_or_else(|| anyhow!("invalid hook path: {}", path.display()))?;
    fs::create_dir_all(parent)
        .with_context(|| format!("failed to create hook dir: {}", parent.display()))?;
    fs::write(&path, pre_commit_hook_contents())
        .with_context(|| format!("failed to write hook: {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms)?;
    }

    Ok(())
}

fn uninstall_pre_commit_hook() -> Result<()> {
    let path = PathBuf::from(".git/hooks/pre-commit");
    if !path.exists() {
        return Ok(());
    }

    let source = fs::read_to_string(&path)
        .with_context(|| format!("failed to read hook: {}", path.display()))?;
    if !source.contains(PRE_COMMIT_HOOK_MARKER) {
        return Err(anyhow!(
            "refusing to remove hook not installed by Pretender: {}",
            path.display()
        ));
    }

    fs::remove_file(&path).with_context(|| format!("failed to remove hook: {}", path.display()))
}

fn pre_commit_hook_contents() -> String {
    format!("#!/usr/bin/env sh\n{PRE_COMMIT_HOOK_MARKER}\nexec pretender check . --staged\n")
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
        run: cargo install --git https://github.com/charly/pretender --locked pretender
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

fn analyze_path(
    path: &Path,
    detector: &RoleDetector,
    config: &Config,
) -> Result<Option<FileReport>> {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("failed to read source file: {}", path.display()))
        }
    };

    let parser = match get_parser(path) {
        Ok(parser) => parser,
        Err(_) => return Ok(None),
    };

    let (module, diagnostics) = parser.parse(path, &source)?;
    let role = detector.detect(path, &source);
    let thresholds = EffectiveThresholds::for_role(role, &config.thresholds);
    let units = module
        .units
        .iter()
        .map(|unit| build_unit_report(unit, &thresholds))
        .collect();

    let mut file_violations = Vec::new();
    push_limit_violation(
        &mut file_violations,
        "file_lines",
        module.lines_total,
        thresholds.file_lines_max,
    );

    Ok(Some(FileReport {
        path: path.display().to_string(),
        role: role_name(role).to_string(),
        diagnostics: diagnostics.into_iter().map(Into::into).collect(),
        file_violations,
        units,
        external_findings: vec![],
    }))
}

fn load_config() -> Result<Config> {
    let path = Path::new("pretender.toml");
    if path.exists() {
        Config::load_from_path(path).map_err(Into::into)
    } else {
        Ok(Config::default())
    }
}

fn warn_if_no_config() {
    if !Path::new("pretender.toml").exists() {
        eprintln!(
            "no pretender.toml found, using defaults — run `pretender init` to configure"
        );
    }
}

fn apply_file_filter(
    files: Vec<PathBuf>,
    allowed: Option<&HashSet<PathBuf>>,
    cwd: &Path,
) -> Vec<PathBuf> {
    let Some(allowed) = allowed else {
        return files;
    };
    files
        .into_iter()
        .filter(|f| {
            let abs = if f.is_absolute() {
                f.clone()
            } else {
                cwd.join(f)
            };
            let canonical = std::fs::canonicalize(&abs).unwrap_or(abs);
            allowed.contains(&canonical)
        })
        .collect()
}

fn collect_input_files(paths: &[PathBuf], config: &Config) -> Result<Vec<PathBuf>> {
    let mut builder = globset::GlobSetBuilder::new();
    for pattern in &config.pretender.exclude {
        let glob = globset::Glob::new(pattern)
            .with_context(|| format!("invalid exclude pattern: {}", pattern))?;
        builder.add(glob);
    }
    let exclude_set = builder.build().context("failed to build exclude GlobSet")?;

    let mut files = Vec::new();
    for path in paths {
        collect_path(path, &exclude_set, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_path(path: &Path, exclude_set: &globset::GlobSet, out: &mut Vec<PathBuf>) -> Result<()> {
    if exclude_set.is_match(path) {
        return Ok(());
    }

    if path.is_file() {
        out.push(path.to_path_buf());
        return Ok(());
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)
            .with_context(|| format!("failed to read directory: {}", path.display()))?
        {
            let entry = entry?;
            collect_path(&entry.path(), exclude_set, out)?;
        }
        return Ok(());
    }

    Err(anyhow!("path does not exist: {}", path.display()))
}

fn build_unit_report(unit: &model::CodeUnit, thresholds: &EffectiveThresholds) -> UnitReport {
    let metrics = MetricValues {
        cyclomatic: metrics::cyclomatic(unit),
        cognitive: metrics::cognitive(unit),
        assertions: unit.assertions,
        function_lines: metrics::function_lines(unit),
        params: metrics::params(unit),
        nesting_max: metrics::nesting_max(unit),
        abc: metrics::abc(unit),
    };

    let mut violations = Vec::new();
    push_limit_violation(
        &mut violations,
        "cyclomatic",
        metrics.cyclomatic,
        thresholds.cyclomatic_max,
    );
    push_limit_violation(
        &mut violations,
        "cognitive",
        metrics.cognitive,
        thresholds.cognitive_max,
    );
    push_limit_violation(
        &mut violations,
        "function_lines",
        metrics.function_lines,
        thresholds.function_lines_max,
    );
    push_limit_violation(
        &mut violations,
        "params",
        metrics.params,
        thresholds.params_max,
    );
    push_limit_violation(
        &mut violations,
        "nesting_max",
        metrics.nesting_max,
        thresholds.nesting_max,
    );
    push_limit_violation_f64(&mut violations, "abc", metrics.abc, thresholds.abc_max);
    if let Some(min) = thresholds.min_assertions {
        push_min_violation(&mut violations, "min_assertions", metrics.assertions, min);
    }

    if unit.is_exported {
        if let Some(max) = thresholds.exported_cyclomatic_max {
            push_limit_violation(
                &mut violations,
                "exported_cyclomatic",
                metrics.cyclomatic,
                max,
            );
        }
        if let Some(max) = thresholds.exported_params_max {
            push_limit_violation(&mut violations, "exported_params", metrics.params, max);
        }
        if let Some(max) = thresholds.exported_lines_max {
            push_limit_violation(
                &mut violations,
                "exported_lines",
                metrics.function_lines,
                max,
            );
        }
    }

    UnitReport {
        name: unit.name.clone(),
        kind: format!("{:?}", unit.kind),
        start_line: unit.span.start_line,
        metrics,
        violations,
    }
}

fn push_limit_violation(
    out: &mut Vec<ViolationReport>,
    metric: &'static str,
    actual: u32,
    max: u32,
) {
    if actual > max {
        out.push(ViolationReport {
            metric: metric.to_string(),
            actual: actual as f64,
            limit: max as f64,
        });
    }
}

fn push_limit_violation_f64(
    out: &mut Vec<ViolationReport>,
    metric: &'static str,
    actual: f64,
    max: u32,
) {
    if actual > max as f64 {
        out.push(ViolationReport {
            metric: metric.to_string(),
            actual,
            limit: max as f64,
        });
    }
}

fn push_min_violation(out: &mut Vec<ViolationReport>, metric: &'static str, actual: u32, min: u32) {
    if actual < min {
        out.push(ViolationReport {
            metric: metric.to_string(),
            actual: actual as f64,
            limit: min as f64,
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Severity {
    Green,
    Yellow,
    Red,
}

struct FileSummary {
    severity: Severity,
    message: String,
}

fn write_human_report(
    sink: &mut dyn Write,
    report: &CheckReport,
    color: bool,
    bands: &Bands,
    mode: Mode,
) -> Result<()> {
    let red = if color { "\u{1b}[31m" } else { "" };
    let yellow = if color { "\u{1b}[33m" } else { "" };
    let green = if color { "\u{1b}[32m" } else { "" };
    let reset = if color { "\u{1b}[0m" } else { "" };

    let blocking = matches!(mode, Mode::Gate);
    let (violation_icon, violation_label, violation_color) = if blocking {
        ("✗", "VIOLATION", red)
    } else {
        ("⚠", "ADVISORY", yellow)
    };

    for file in &report.files {
        let summary = summarize_file(file, bands);
        let (icon, accent) = match summary.severity {
            Severity::Green => ("✓", green),
            Severity::Yellow => ("⚠", yellow),
            Severity::Red => (violation_icon, if blocking { red } else { yellow }),
        };

        writeln!(
            sink,
            "{accent}{icon}{reset} {} — {}",
            file.path, summary.message
        )?;
        writeln!(sink, "  role: {}", file.role)?;

        for violation in &file.file_violations {
            writeln!(
                sink,
                "  {violation_color}{violation_label}{reset} {} {} > {}",
                violation.metric, violation.actual, violation.limit,
            )?;
        }
        for unit in &file.units {
            writeln!(
                sink,
                "  {}: cyclomatic={}, cognitive={}, assertions={}, function_lines={}, params={}, nesting_max={}, abc={:.2}",
                unit.name,
                unit.metrics.cyclomatic,
                unit.metrics.cognitive,
                unit.metrics.assertions,
                unit.metrics.function_lines,
                unit.metrics.params,
                unit.metrics.nesting_max,
                unit.metrics.abc,
            )?;
            for violation in &unit.violations {
                writeln!(
                    sink,
                    "    {violation_color}{violation_label}{reset} {} {} > {}",
                    violation.metric, violation.actual, violation.limit,
                )?;
            }
        }
        for diagnostic in &file.diagnostics {
            eprintln!("  {:?}: {}", diagnostic.severity, diagnostic.message);
        }
        for finding in &file.external_findings {
            let code = finding.code.as_deref().unwrap_or("-");
            writeln!(
                sink,
                "  {red}EXTERNAL{reset} {} {} line {}: {}",
                finding.source, code, finding.line, finding.message
            )?;
        }
    }
    Ok(())
}

fn summarize_file(file: &FileReport, bands: &Bands) -> FileSummary {
    let mut summary = FileSummary {
        severity: Severity::Green,
        message: "all green".to_string(),
    };

    for diagnostic in &file.diagnostics {
        if diagnostic.severity == "Error" {
            consider_summary(
                &mut summary,
                Severity::Red,
                format!("parse error: {}", diagnostic.message),
            );
        }
    }

    for violation in &file.file_violations {
        consider_summary(
            &mut summary,
            Severity::Red,
            format!(
                "{} {:.0} > {:.0}",
                violation.metric, violation.actual, violation.limit
            ),
        );
    }

    for unit in &file.units {
        consider_banded_metric(
            &mut summary,
            &unit.name,
            "cyclomatic",
            unit.metrics.cyclomatic,
            bands.cyclomatic,
        );
        consider_banded_metric(
            &mut summary,
            &unit.name,
            "cognitive",
            unit.metrics.cognitive,
            bands.cognitive,
        );

        for violation in &unit.violations {
            if violation.metric == "cyclomatic" || violation.metric == "cognitive" {
                continue;
            }
            consider_summary(
                &mut summary,
                Severity::Red,
                format!(
                    "{}(): {} {:.0} > {:.0}",
                    unit.name, violation.metric, violation.actual, violation.limit
                ),
            );
        }
    }

    summary
}

fn consider_banded_metric(
    summary: &mut FileSummary,
    unit_name: &str,
    metric: &'static str,
    actual: u32,
    band: Option<Band>,
) {
    let severity = band_severity(actual, band);
    if severity == Severity::Green {
        return;
    }

    consider_summary(
        summary,
        severity,
        format!(
            "{}(): {} {} ({})",
            unit_name,
            metric,
            actual,
            severity_label(severity)
        ),
    );
}

fn consider_summary(summary: &mut FileSummary, severity: Severity, message: String) {
    if severity > summary.severity {
        summary.severity = severity;
        summary.message = message;
    }
}

fn band_severity(actual: u32, band: Option<Band>) -> Severity {
    match band {
        Some(band) if actual > band.yellow => Severity::Red,
        Some(band) if actual > band.green => Severity::Yellow,
        _ => Severity::Green,
    }
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Green => "green",
        Severity::Yellow => "yellow",
        Severity::Red => "red",
    }
}

fn color_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    io::stdout().is_terminal()
}

fn write_markdown_report(sink: &mut dyn Write, report: &CheckReport, bands: &Bands) -> Result<()> {
    writeln!(sink, "# Pretender report")?;
    writeln!(sink)?;

    for file in &report.files {
        let summary = summarize_file(file, bands);
        let icon = match summary.severity {
            Severity::Green => "✓",
            Severity::Yellow => "⚠",
            Severity::Red => "✗",
        };
        writeln!(sink, "## `{}` {} {}", file.path, icon, summary.message)?;
        writeln!(sink)?;
        writeln!(sink, "- Role: `{}`", file.role)?;

        for violation in &file.file_violations {
            writeln!(
                sink,
                "- File violation: `{}` actual `{:.0}` > limit `{:.0}`",
                violation.metric, violation.actual, violation.limit
            )?;
        }

        for unit in &file.units {
            writeln!(
                sink,
                "- `{}`: cyclomatic `{}`, cognitive `{}`, assertions `{}`, function_lines `{}`, params `{}`, nesting_max `{}`, abc `{:.2}`",
                unit.name,
                unit.metrics.cyclomatic,
                unit.metrics.cognitive,
                unit.metrics.assertions,
                unit.metrics.function_lines,
                unit.metrics.params,
                unit.metrics.nesting_max,
                unit.metrics.abc,
            )?;
            for violation in &unit.violations {
                writeln!(
                    sink,
                    "  - violation: `{}` actual `{:.0}` > limit `{:.0}`",
                    violation.metric, violation.actual, violation.limit
                )?;
            }
        }

        if !file.diagnostics.is_empty() {
            writeln!(sink)?;
            writeln!(sink, "### Diagnostics")?;
            for diagnostic in &file.diagnostics {
                writeln!(sink, "- {}: {}", diagnostic.severity, diagnostic.message)?;
            }
        }

        writeln!(sink)?;
    }

    Ok(())
}

fn write_html_report(sink: &mut dyn Write, report: &CheckReport, bands: &Bands) -> Result<()> {
    writeln!(sink, "<!doctype html>")?;
    writeln!(sink, "<html><head><meta charset=\"utf-8\"><title>Pretender report</title><style>body{{font-family:system-ui,sans-serif;max-width:960px;margin:2rem auto;padding:0 1rem}} .green{{color:#18794e}} .yellow{{color:#9a6700}} .red{{color:#cf222e}} code{{background:#f6f8fa;padding:.1rem .3rem;border-radius:4px}} ul{{line-height:1.5}}</style></head><body>")?;
    writeln!(sink, "<h1>Pretender report</h1>")?;

    for file in &report.files {
        let summary = summarize_file(file, bands);
        let (icon, class_name) = match summary.severity {
            Severity::Green => ("✓", "green"),
            Severity::Yellow => ("⚠", "yellow"),
            Severity::Red => ("✗", "red"),
        };
        writeln!(
            sink,
            "<section><h2><code>{}</code> <span class=\"{}\">{} {}</span></h2>",
            html_escape(&file.path),
            class_name,
            icon,
            html_escape(&summary.message)
        )?;
        writeln!(
            sink,
            "<p>Role: <code>{}</code></p><ul>",
            html_escape(&file.role)
        )?;

        for violation in &file.file_violations {
            writeln!(
                sink,
                "<li>File violation: <code>{}</code> actual <code>{:.0}</code> &gt; limit <code>{:.0}</code></li>",
                html_escape(&violation.metric),
                violation.actual,
                violation.limit
            )?;
        }

        for unit in &file.units {
            writeln!(
                sink,
                "<li><code>{}</code>: cyclomatic <code>{}</code>, cognitive <code>{}</code>, assertions <code>{}</code>, function_lines <code>{}</code>, params <code>{}</code>, nesting_max <code>{}</code>, abc <code>{:.2}</code>",
                html_escape(&unit.name),
                unit.metrics.cyclomatic,
                unit.metrics.cognitive,
                unit.metrics.assertions,
                unit.metrics.function_lines,
                unit.metrics.params,
                unit.metrics.nesting_max,
                unit.metrics.abc,
            )?;
            if !unit.violations.is_empty() {
                writeln!(sink, "<ul>")?;
                for violation in &unit.violations {
                    writeln!(
                        sink,
                        "<li>violation: <code>{}</code> actual <code>{:.0}</code> &gt; limit <code>{:.0}</code></li>",
                        html_escape(&violation.metric),
                        violation.actual,
                        violation.limit
                    )?;
                }
                writeln!(sink, "</ul>")?;
            }
            writeln!(sink, "</li>")?;
        }

        writeln!(sink, "</ul>")?;
        if !file.diagnostics.is_empty() {
            writeln!(sink, "<h3>Diagnostics</h3><ul>")?;
            for diagnostic in &file.diagnostics {
                writeln!(
                    sink,
                    "<li>{}: {}</li>",
                    html_escape(&diagnostic.severity),
                    html_escape(&diagnostic.message)
                )?;
            }
            writeln!(sink, "</ul>")?;
        }
        writeln!(sink, "</section>")?;
    }

    writeln!(sink, "</body></html>")?;
    Ok(())
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn write_json_report(sink: &mut dyn Write, report: &CheckReport) -> Result<()> {
    serde_json::to_writer_pretty(&mut *sink, report)?;
    writeln!(sink)?;
    Ok(())
}

fn write_sarif_report(sink: &mut dyn Write, report: &CheckReport) -> Result<()> {
    use serde_sarif::sarif;
    use std::collections::HashMap;

    let mut rule_index: HashMap<String, i64> = HashMap::new();
    let mut rules: Vec<sarif::ReportingDescriptor> = Vec::new();
    let mut results: Vec<sarif::Result> = Vec::new();

    let push_result = |rules: &mut Vec<sarif::ReportingDescriptor>,
                       rule_index: &mut HashMap<String, i64>,
                       results: &mut Vec<sarif::Result>,
                       violation: &ViolationReport,
                       file_path: &str,
                       start_line: i64,
                       label: &str| {
        let idx = if let Some(&i) = rule_index.get(&violation.metric) {
            i
        } else {
            let i = rules.len() as i64;
            rules.push(
                sarif::ReportingDescriptor::builder()
                    .id(&violation.metric)
                    .build(),
            );
            rule_index.insert(violation.metric.clone(), i);
            i
        };
        let message_text = format!(
            "{} in {} exceeds limit: actual={:.0}, limit={:.0}",
            violation.metric, label, violation.actual, violation.limit
        );
        let location = sarif::Location::builder()
            .physical_location(
                sarif::PhysicalLocation::builder()
                    .artifact_location(sarif::ArtifactLocation::builder().uri(file_path).build())
                    .region(sarif::Region::builder().start_line(start_line).build())
                    .build(),
            )
            .build();
        results.push(
            sarif::Result::builder()
                .rule_id(&violation.metric)
                .rule_index(idx)
                .message(sarif::Message::builder().text(message_text).build())
                .locations(vec![location])
                .level(sarif::ResultLevel::Warning)
                .build(),
        );
    };

    for file in &report.files {
        for violation in &file.file_violations {
            push_result(
                &mut rules,
                &mut rule_index,
                &mut results,
                violation,
                &file.path,
                1,
                &file.path,
            );
        }
        for unit in &file.units {
            for violation in &unit.violations {
                push_result(
                    &mut rules,
                    &mut rule_index,
                    &mut results,
                    violation,
                    &file.path,
                    unit.start_line as i64,
                    &unit.name,
                );
            }
        }
    }

    let tool_component = sarif::ToolComponent::builder()
        .name("pretender")
        .rules(rules)
        .build();
    let run = sarif::Run::builder()
        .tool(tool_component)
        .results(results)
        .build();
    let sarif_log = sarif::Sarif::builder()
        .version(sarif::Version::V2_1_0.to_string())
        .schema(sarif::SCHEMA_URL)
        .runs(vec![run])
        .build();

    serde_json::to_writer_pretty(&mut *sink, &sarif_log)?;
    writeln!(sink)?;
    Ok(())
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::App => "app",
        Role::Library => "library",
        Role::Test => "test",
        Role::Script => "script",
        Role::Generated => "generated",
        Role::Vendor => "vendor",
    }
}

fn get_parser(path: &Path) -> Result<Box<dyn model::Parser>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("missing file extension for path: {}", path.display()))?;

    match ext {
        "c" | "h" => Ok(Box::new(c::CParser)),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Ok(Box::new(cpp::CppParser)),
        "go" => Ok(Box::new(go::GoParser)),
        "java" => Ok(Box::new(java::JavaParser)),
        "js" | "jsx" | "mjs" | "cjs" => Ok(Box::new(javascript::JavaScriptParser)),
        "py" => Ok(Box::new(python::PythonParser)),
        "rb" => Ok(Box::new(ruby::RubyParser)),
        "rs" => Ok(Box::new(rust::RustParser)),
        "ts" | "mts" => Ok(Box::new(typescript::TypeScriptParser)),
        "tsx" | "cts" => Ok(Box::new(typescript::TypeScriptXParser)),
        _ => Err(anyhow!("unsupported file extension '.{}'", ext)),
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct CheckReport {
    files: Vec<FileReport>,
}

#[derive(Debug, Serialize, Deserialize)]
struct FileReport {
    path: String,
    role: String,
    diagnostics: Vec<DiagnosticReport>,
    file_violations: Vec<ViolationReport>,
    units: Vec<UnitReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    external_findings: Vec<external_plugin::ExternalFinding>,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
struct UnitReport {
    name: String,
    kind: String,
    start_line: u32,
    metrics: MetricValues,
    violations: Vec<ViolationReport>,
}

#[derive(Debug, Serialize, Deserialize)]
struct MetricValues {
    cyclomatic: u32,
    cognitive: u32,
    assertions: u32,
    function_lines: u32,
    params: u32,
    nesting_max: u32,
    abc: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ViolationReport {
    metric: String,
    actual: f64,
    limit: f64,
}

fn emit_history_events(
    report: &CheckReport,
    config: &config::Config,
    format: ReportFormat,
    writing_to_stdout: bool,
    sink: &mut dyn Write,
) {
    let timestamp = history::now_iso8601();
    let run_id = history::make_run_id();
    let mode_str = format!("{:?}", config.pretender.mode).to_ascii_lowercase();
    let new_events = cognitive_max_events(report, &run_id, &timestamp, &mode_str);
    if new_events.is_empty() {
        return;
    }
    let store = history::EventStore::new(Path::new(".pretender"));
    let all_events = match store.append_and_prune(&new_events) {
        Ok(events) => events,
        Err(err) => {
            eprintln!("pretender: history write failed: {err}");
            return;
        }
    };
    let summary = history::compute_summary(&all_events);
    let _ = store.persist_summary(&summary);
    if matches!(format, ReportFormat::Human)
        && (!summary.top_hotspots.is_empty() || !summary.top_patterns.is_empty())
    {
        let color = writing_to_stdout && color_enabled();
        let _ = write_recurrence_hints(sink, &summary, color);
    }
}

fn cognitive_max_events(
    report: &CheckReport,
    run_id: &str,
    timestamp: &str,
    mode: &str,
) -> Vec<history::ViolationEvent> {
    // TODO(Phase 2): extend to cyclomatic_max, params_max, nesting_max,
    // function_lines_max, abc_max — see feedback-loop-design.md §Phase 2.
    let mut events = Vec::new();
    for file in &report.files {
        // Normalize away leading "./" so fingerprints are stable regardless of
        // how the CLI path was spelled (e.g. "src/foo.rs" vs "./src/foo.rs").
        let path = file
            .path
            .strip_prefix("./")
            .unwrap_or(&file.path)
            .to_string();
        for unit in &file.units {
            for violation in &unit.violations {
                if violation.metric != "cognitive" {
                    continue;
                }
                events.push(history::ViolationEvent {
                    schema_version: 1,
                    timestamp: timestamp.to_string(),
                    run_id: run_id.to_string(),
                    mode: mode.to_string(),
                    path: path.clone(),
                    unit_name: Some(unit.name.clone()),
                    role: file.role.clone(),
                    rule_key: "cognitive_max".to_string(),
                    metric_family: "complexity".to_string(),
                    scope: "unit".to_string(),
                    // Phase 1: only threshold violations (red); yellow-band and
                    // gate-fail severity levels will be added in Phase 2.
                    severity: "red".to_string(),
                    actual: violation.actual,
                    limit: violation.limit,
                    delta: violation.actual - violation.limit,
                    fingerprint: format!("{}::{}::cognitive_max", path, unit.name),
                });
            }
        }
    }
    events
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

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();
    cli.command.run()
}
