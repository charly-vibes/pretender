mod c;
mod config;
mod cpp;
mod engine;
mod go;
mod java;
mod javascript;
mod metrics;
mod model;
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
use std::fs;
use std::io::{self, IsTerminal, Write};
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
    /// Skip prompts, use best-guess defaults
    #[arg(long)]
    defaults: bool,
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
    /// Only check git-staged files
    #[arg(long)]
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
    #[arg(long)]
    score_min: Option<u32>,
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
            Commands::Init(_) => not_implemented("init", "pretender-rl3"),
            Commands::Check(args) => args.run(),
            Commands::Complexity(args) => args.run(),
            Commands::Report(args) => args.run(),
            Commands::Duplication(_) => not_implemented("duplication", "pretender-xgn"),
            Commands::Mutation(_) => not_implemented("mutation", "pretender-238"),
            Commands::Hooks(_) => not_implemented("hooks", "pretender-hay"),
            Commands::Ci(_) => not_implemented("ci generate", "pretender-fb3"),
            Commands::Plugins(_) => not_implemented("plugins", "pretender-07m"),
            Commands::Explain(_) => not_implemented("explain", "pretender-vuc"),
        }
    }
}

fn not_implemented(name: &str, tracker: &str) -> Result<ExitCode> {
    eprintln!("pretender {name}: not yet implemented (tracked: {tracker})");
    Ok(ExitCode::from(NOT_IMPLEMENTED_EXIT))
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
        if self.staged || self.diff_only || self.diff_base.is_some() {
            return not_implemented("check --staged/--diff-only/--diff-base", "pretender-a80");
        }
        if matches!(self.format, ReportFormat::Junit | ReportFormat::Markdown) {
            return not_implemented(
                &format!("check --format {:?}", self.format).to_lowercase(),
                "pretender-t2m",
            );
        }

        let mut config = load_config()?;
        if let Some(mode) = self.mode {
            config.pretender.mode = mode.into();
        }
        let detector = RoleDetector::new(&config).context("failed to initialize role detector")?;
        let files = collect_input_files(&self.paths, &config)?;

        let mut reports: Vec<FileReport> = files
            .par_iter()
            .filter_map(|path| analyze_path(path, &detector, &config).transpose())
            .collect::<Result<_>>()?;
        reports.sort_by(|a, b| a.path.cmp(&b.path));

        let report = CheckReport { files: reports };
        let writing_to_stdout = self.output.is_none();
        let mut sink = open_report_sink(self.output.as_deref())?;
        match self.format {
            ReportFormat::Human => {
                let color = writing_to_stdout && color_enabled();
                write_human_report(sink.as_mut(), &report, color, &config.bands)?;
            }
            ReportFormat::Json => {
                write_json_report(sink.as_mut(), &report)?;
                persist_last_check_report(&report)?;
            }
            ReportFormat::Sarif => write_sarif_report(sink.as_mut(), &report)?,
            _ => unreachable!("junit/markdown handled above"),
        }
        sink.flush().context("failed to flush report output")?;

        Ok(decide_exit_code(&report, config.pretender.mode))
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
                write_human_report(sink.as_mut(), &report, color, &config.bands)?;
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
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read source file: {}", path.display()))?;

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
) -> Result<()> {
    let red = if color { "\u{1b}[31m" } else { "" };
    let yellow = if color { "\u{1b}[33m" } else { "" };
    let green = if color { "\u{1b}[32m" } else { "" };
    let reset = if color { "\u{1b}[0m" } else { "" };

    for file in &report.files {
        let summary = summarize_file(file, bands);
        let (icon, accent) = match summary.severity {
            Severity::Green => ("✓", green),
            Severity::Yellow => ("⚠", yellow),
            Severity::Red => ("✗", red),
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
                "  {red}VIOLATION{reset} {} {} > {}",
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
                    "    {red}VIOLATION{reset} {} {} > {}",
                    violation.metric, violation.actual, violation.limit,
                )?;
            }
        }
        for diagnostic in &file.diagnostics {
            eprintln!("  {:?}: {}", diagnostic.severity, diagnostic.message);
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

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();
    cli.command.run()
}
