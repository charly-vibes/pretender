//! Event construction from violation findings.

use crate::check::gate::{violation_finding_id, violation_findings, ViolationFinding};
use crate::report::CheckReport;
use pretender::config;
use pretender::history;

pub(super) fn all_violation_events(
    report: &CheckReport,
    run_id: &str,
    timestamp: &str,
    config: &config::Config,
) -> Vec<history::ViolationEvent> {
    let mode = format!("{:?}", config.pretender.mode).to_ascii_lowercase();
    violation_findings(report)
        .into_iter()
        .map(|finding| violation_event(finding, run_id, timestamp, &mode))
        .collect()
}

/// Build the history event for one violation finding.
fn violation_event(
    finding: ViolationFinding,
    run_id: &str,
    timestamp: &str,
    mode: &str,
) -> history::ViolationEvent {
    history::ViolationEvent {
        measure: event_measure(&finding),
        origin: event_origin(run_id, timestamp, mode),
        finding: event_finding(finding),
    }
}

/// Fingerprinted magnitude of the violation.
fn event_measure(finding: &ViolationFinding) -> history::EventMeasure {
    history::EventMeasure {
        actual: finding.actual,
        limit: finding.limit,
        delta: finding.actual - finding.limit,
        fingerprint: violation_finding_id(finding),
    }
}

/// Run-scoped origin metadata.
fn event_origin(run_id: &str, timestamp: &str, mode: &str) -> history::EventOrigin {
    history::EventOrigin {
        schema_version: 1,
        timestamp: timestamp.to_string(),
        run_id: run_id.to_string(),
        mode: mode.to_string(),
    }
}

/// What was violated, where, and at which severity.
fn event_finding(finding: ViolationFinding) -> history::EventFinding {
    history::EventFinding {
        path: finding.path,
        role: finding.role,
        rule_key: finding.rule_key,
        metric_family: metric_to_family(&finding.metric).to_string(),
        scope: finding_scope(&finding.unit_name).to_string(),
        unit_name: finding.unit_name,
        severity: "red".to_string(),
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

/// Unit-scoped when the finding carries a unit name, file-scoped otherwise.
fn finding_scope(unit_name: &Option<String>) -> &'static str {
    if unit_name.is_some() {
        "unit"
    } else {
        "file"
    }
}

/// Stable finding IDs (fingerprints) of the given events, in order.
pub(super) fn finding_ids(events: &[history::ViolationEvent]) -> Vec<String> {
    events
        .iter()
        .map(|e| e.measure.fingerprint.clone())
        .collect()
}
