//! Check-report writers for the CLI: human, markdown, HTML, JSON, and SARIF.
//!
//! Each writer renders a finished `CheckReport` snapshot to a sink owned by
//! the caller, which the CLI selects by report format. Format modules own
//! their rendering; `summary` owns the severity/band summarization and
//! metric formatting shared by the human, markdown, and HTML writers.
//!
//! # Responsibilities
//! Re-export the five writer entry points at `crate::report` and keep the
//! JSON envelope writer; delegate all other formats to `human`, `markdown`,
//! `html`, and `sarif`.
//!
//! # Rationale
//! Extracted from `main.rs` (epic pretender-5zo) and split into submodule
//! seams so no file exceeds the gate's interim file-lines limit; bodies
//! moved verbatim to keep output bytes and ordering exactly unchanged.

mod html;
mod human;
mod markdown;
mod sarif;
mod summary;

use crate::CheckReport;
use anyhow::Result;
use genesis::envelope::{Envelope, EnvelopeKind, Warning};
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
    let remediation_text = "set advisory_until or switch to mode=gate";
    let mut env = Envelope::success(version, kind, report, vec![], vec![]);
    // GH #35: a lease fail-closed exits 1, so the envelope must not claim
    // ok:true — surface the reason as a structured warning instead.
    if let Some(message) = lease_warning {
        env.ok = false;
        env.warnings.push(Warning {
            rule_name: "advisory-lease".to_string(),
            entity_id: None,
            message,
            suggested_remediation: Some(remediation_text.to_string()),
        });
    }
    serde_json::to_writer_pretty(&mut *sink, &env)?;
    writeln!(sink)?;
    Ok(())
}
