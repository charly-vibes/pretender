//! Check-report writers for the CLI: human, markdown, HTML, JSON, and SARIF.
//!
//! Each writer renders a finished `CheckReport` snapshot to a sink owned by
//! the caller, which the CLI selects by report format. Format modules own
//! their rendering; `summary` owns the severity/band summarization and
//! metric formatting shared by the human, markdown, and HTML writers.
//!
//! # Responsibilities
//! Re-export the five writer entry points at `crate::report`, keep the
//! JSON envelope writer, and own the report data model (`CheckReport` and
//! its members) shared by writers, the check pipeline, and history
//! emission; delegate all other formats to `human`, `markdown`, `html`,
//! and `sarif`.
//!
//! # Rationale
//! Extracted from `main.rs` (epic pretender-5zo) and split into submodule
//! seams so no file exceeds the gate's interim file-lines limit; bodies
//! moved verbatim to keep output bytes and ordering exactly unchanged.
//! The report data model moved here from the bin root in pretender-5zo.2.5
//! (serde shapes untouched, so JSON output stays byte-identical).

mod html;
mod human;
mod markdown;
mod sarif;
mod summary;

use anyhow::Result;
use genesis::envelope::{Envelope, EnvelopeKind, Warning};
use pretender::{external_plugin, history, model, test_report};
use serde::{Deserialize, Serialize};
use std::io::Write;

pub(crate) use html::write_html_report;
pub(crate) use human::{write_human_report, HumanOpts};
pub(crate) use markdown::write_markdown_report;
pub(crate) use sarif::write_sarif_report;

pub(crate) fn write_json_report(
    sink: &mut dyn Write,
    report: &CheckReport,
    lease_warning: Option<String>,
) -> Result<()> {
    let version = env!("CARGO_PKG_VERSION");
    let kind = EnvelopeKind::Check;
    let mut env = Envelope::success(version, kind, report, vec![], vec![]);
    apply_lease_warning(&mut env, lease_warning);
    serde_json::to_writer_pretty(&mut *sink, &env)?;
    writeln!(sink)?;
    Ok(())
}

/// GH #35: a lease fail-closed exits 1, so the envelope must not claim
/// ok:true — surface the reason as a structured warning instead.
fn apply_lease_warning<T: Serialize>(env: &mut Envelope<T>, lease_warning: Option<String>) {
    if let Some(message) = lease_warning {
        env.ok = false;
        env.warnings.push(Warning {
            rule_name: "advisory-lease".to_string(),
            entity_id: None,
            message,
            suggested_remediation: Some(LEASE_REMEDIATION.to_string()),
        });
    }
}

/// Remediation hint attached to the advisory-lease warning.
const LEASE_REMEDIATION: &str = "set advisory_until or switch to mode=gate";

/// The finished check result: one report per `pretender check` run and
/// the persisted shape of `.pretender/last-check.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CheckReport {
    pub(crate) files: Vec<FileReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) modules: Vec<ModuleReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) cycles: Vec<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) test_findings: Vec<test_report::TestDurationFinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) history: Option<history::HistorySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct FileReport {
    pub(crate) path: String,
    pub(crate) role: String,
    pub(crate) diagnostics: Vec<DiagnosticReport>,
    pub(crate) file_violations: Vec<ViolationReport>,
    pub(crate) units: Vec<UnitReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) external_findings: Vec<external_plugin::ExternalFinding>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) mock_findings: Vec<MockFindingReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct DiagnosticReport {
    pub(crate) severity: String,
    pub(crate) message: String,
    pub(crate) start_line: Option<u32>,
    pub(crate) end_line: Option<u32>,
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
pub(crate) struct UnitReport {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) start_line: u32,
    pub(crate) metrics: MetricValues,
    pub(crate) violations: Vec<ViolationReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MetricValues {
    pub(crate) cyclomatic: u32,
    pub(crate) cognitive: u32,
    pub(crate) assertions: u32,
    pub(crate) function_lines: u32,
    pub(crate) params: u32,
    pub(crate) nesting_max: u32,
    pub(crate) abc: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ViolationReport {
    pub(crate) metric: String,
    pub(crate) actual: f64,
    pub(crate) limit: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) line: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ModuleReport {
    pub(crate) path: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) coupling_violations: Vec<CouplingViolationReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CouplingViolationReport {
    pub(crate) metric: String,
    pub(crate) actual: f64,
    pub(crate) limit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MockFindingReport {
    pub(crate) name: String,
    pub(crate) line: u32,
    pub(crate) kind: String,
}

/// Test fixture shared by the JSON and SARIF writer tests.
#[cfg(test)]
fn violation(line: Option<u32>) -> ViolationReport {
    ViolationReport {
        metric: "cyclomatic".to_string(),
        actual: 12.0,
        limit: 10.0,
        line,
    }
}

#[cfg(test)]
mod violation_report_tests {
    use super::*;

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
}
