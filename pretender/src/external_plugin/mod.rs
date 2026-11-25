//! Purpose: external metric-plugin integration — load plugin TOML configs,
//! run the configured commands over applicable files, and parse their JSON
//! output into findings.
//! Responsibilities: config types, metrics-directory discovery, command
//! invocation (`loader`), and mapping stdout to per-path findings via the
//! configured field mapping (`parse`).
//! Rationale: plugins are optional best-effort integrations; failures are
//! reported to stderr and skipped rather than aborting the run.

mod loader;
mod parse;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use loader::{load_plugins, run_plugins};

#[derive(Debug, Clone, Deserialize)]
pub struct ExternalPluginConfig {
    pub name: String,
    pub extensions: Vec<String>,
    pub command: Vec<String>,
    pub parser: ParserKind,
    pub mapping: FieldMapping,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ParserKind {
    Json,
    JsonLines,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FieldMapping {
    pub path: String,
    pub line: String,
    pub message: String,
    #[serde(default)]
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalFinding {
    pub source: String,
    pub line: u32,
    pub message: String,
    pub code: Option<String>,
}

/// Returns the directory for external metric plugin configs.
///
/// Priority: `PRETENDER_METRICS_DIR` env var → `$XDG_CONFIG_HOME/pretender/metrics`
/// → `~/.config/pretender/metrics`.
pub fn default_metrics_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("PRETENDER_METRICS_DIR") {
        return PathBuf::from(dir);
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    base.join("pretender").join("metrics")
}
