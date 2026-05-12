mod config;
mod engine;
mod metrics;
mod model;
mod python;
mod roles;

use crate::config::Config;
use crate::model::Metric;
use crate::roles::{EffectiveThresholds, Role, RoleDetector};
use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;
use std::cmp::Reverse;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "pretender")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show cyclomatic complexity for each function, sorted worst-first
    Complexity(ComplexityArgs),
    /// Analyze files and emit a report
    Check(CheckArgs),
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
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ReportFormat {
    Human,
    Json,
}

trait Executable {
    fn run(&self) -> Result<()>;
}

impl Executable for Commands {
    fn run(&self) -> Result<()> {
        match self {
            Commands::Complexity(args) => args.run(),
            Commands::Check(args) => args.run(),
        }
    }
}

impl Executable for ComplexityArgs {
    fn run(&self) -> Result<()> {
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
        Ok(())
    }
}

impl Executable for CheckArgs {
    fn run(&self) -> Result<()> {
        let config = load_config()?;
        let detector = RoleDetector::new(&config).context("failed to initialize role detector")?;
        let files = collect_input_files(&self.paths)?;

        let reports: Vec<FileReport> = files
            .into_iter()
            .filter_map(|path| analyze_path(&path, &detector, &config).transpose())
            .collect::<Result<_>>()?;

        let report = CheckReport { files: reports };
        match self.format {
            ReportFormat::Human => print_human_report(&report),
            ReportFormat::Json => print_json_report(&report)?,
        }

        Ok(())
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

    Ok(Some(FileReport {
        path: path.display().to_string(),
        role: role_name(role),
        diagnostics: diagnostics.into_iter().map(Into::into).collect(),
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

fn collect_input_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        collect_path(path, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

fn collect_path(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_file() {
        out.push(path.to_path_buf());
        return Ok(());
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)
            .with_context(|| format!("failed to read directory: {}", path.display()))?
        {
            let entry = entry?;
            collect_path(&entry.path(), out)?;
        }
        return Ok(());
    }

    Err(anyhow!("path does not exist: {}", path.display()))
}

fn build_unit_report(unit: &model::CodeUnit, thresholds: &EffectiveThresholds) -> UnitReport {
    let metrics = MetricValues {
        cyclomatic: metrics::cyclomatic(unit),
        cognitive: metrics::cognitive(unit),
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

    UnitReport {
        name: unit.name.clone(),
        kind: format!("{:?}", unit.kind),
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
            metric,
            actual: actual as f64,
            limit: max as f64,
        });
    }
}

fn print_human_report(report: &CheckReport) {
    for file in &report.files {
        println!("{} [{}]", file.path, file.role);
        for unit in &file.units {
            println!(
                "  {}: cyclomatic={}, cognitive={}, function_lines={}, params={}, nesting_max={}, abc={:.2}",
                unit.name,
                unit.metrics.cyclomatic,
                unit.metrics.cognitive,
                unit.metrics.function_lines,
                unit.metrics.params,
                unit.metrics.nesting_max,
                unit.metrics.abc,
            );
        }
        for diagnostic in &file.diagnostics {
            eprintln!("  {:?}: {}", diagnostic.severity, diagnostic.message);
        }
    }
}

fn print_json_report(report: &CheckReport) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(report)?);
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

    let mut registry = model::ParserRegistry::new();
    registry.register(Box::new(python::PythonParser));

    registry
        .get_for_extension(ext)
        .map(|_p| match ext {
            "py" => Box::new(python::PythonParser) as Box<dyn model::Parser>,
            _ => unreachable!(
                "registry returned parser for unsupported extension .{}",
                ext
            ),
        })
        .ok_or_else(|| anyhow!("unsupported file extension '.{}'", ext))
}

#[derive(Debug, Serialize)]
struct CheckReport {
    files: Vec<FileReport>,
}

#[derive(Debug, Serialize)]
struct FileReport {
    path: String,
    role: &'static str,
    diagnostics: Vec<DiagnosticReport>,
    units: Vec<UnitReport>,
}

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
struct UnitReport {
    name: String,
    kind: String,
    metrics: MetricValues,
    violations: Vec<ViolationReport>,
}

#[derive(Debug, Serialize)]
struct MetricValues {
    cyclomatic: u32,
    cognitive: u32,
    function_lines: u32,
    params: u32,
    nesting_max: u32,
    abc: f64,
}

#[derive(Debug, Serialize)]
struct ViolationReport {
    metric: &'static str,
    actual: f64,
    limit: f64,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.command.run()
}
