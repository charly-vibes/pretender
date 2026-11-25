//! CLI support infrastructure: config loading, report sinks, and the
//! persisted last-check report.
//!
//! The small file/env plumbing every command runner shares: loading and
//! validating `pretender.toml`, choosing an output sink, and reading or
//! writing `.pretender/last-check.json` for the `report` command.
//!
//! # Responsibilities
//! `load_config` (with its validation tail split out so every function
//! stays within the size limit), `warn_if_no_config`, `color_enabled`,
//! `open_report_sink`, and the last-check persistence pair.
//!
//! # Rationale
//! Extracted from `main.rs` (pretender-5zo.2.5) as the CLI-side home for
//! this infrastructure: it reads and writes process-local state on behalf
//! of command runners, and its bodies moved with only size-forced splits
//! so error messages and defaults stay byte-identical.
use crate::cli::args::{LongReportFormat, ReportArgs};
use crate::cli::Executable;
use crate::report::{
    write_html_report, write_human_report, write_markdown_report, CheckReport, HumanOpts,
};
use anyhow::{anyhow, Context, Result};
use genesis::config::{ConfigFile, ConfigStore, ValidationSeverity};
use pretender::config::{self, Config};
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

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

/// Load `pretender.toml`, falling back to defaults when absent or empty.
pub(crate) fn load_config() -> Result<Config> {
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
    validate_config(parsed)
}

/// Validate a parsed config, failing on the first error-severity entry.
fn validate_config(parsed: Config) -> Result<Config> {
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

pub(crate) fn warn_if_no_config() {
    if !Path::new("pretender.toml").exists() {
        eprintln!("no pretender.toml found, using defaults — run `pretender init` to configure");
    }
}

pub(crate) fn color_enabled() -> bool {
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    io::stdout().is_terminal()
}

pub(crate) fn open_report_sink(path: Option<&Path>) -> Result<Box<dyn Write>> {
    match path {
        Some(path) => {
            let file = fs::File::create(path)
                .with_context(|| format!("failed to open output path: {}", path.display()))?;
            Ok(Box::new(io::BufWriter::new(file)))
        }
        None => Ok(Box::new(io::stdout().lock())),
    }
}

fn last_check_report_path() -> PathBuf {
    PathBuf::from(".pretender/last-check.json")
}

pub(crate) fn persist_last_check_report(report: &CheckReport) -> Result<()> {
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

pub(crate) fn load_last_check_report() -> Result<CheckReport> {
    let path = last_check_report_path();
    let source = fs::read_to_string(&path)
        .with_context(|| format!("failed to read last check report: {}", path.display()))?;
    serde_json::from_str(&source).context("failed to parse last check report JSON")
}
