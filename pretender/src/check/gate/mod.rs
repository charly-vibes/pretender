//! Check gate: exit-code decision, advisory-lease fail-closed, baseline
//! location, and test-report resolution.
//!
//! Decides whether a completed scan exits success or failure, resolves the
//! optional JUnit report used for test-duration analysis, and hosts the
//! gate-baseline path constant.
//!
//! # Responsibilities
//! `decide_exit_code` is the single gate verdict (lease, violations,
//! coupling, cycles, durations); `resolve_test_report_path` runs the
//! optional `test_cmd` and locates its report. Lease diagnostics and the
//! lease matrix tests live here so the fail-closed contract stays in one
//! file.
//!
//! # Rationale
//! Extracted verbatim from main.rs (pretender-5zo.2.2); the two long
//! functions were split into verbatim-behaving helpers while moving so
//! every function lands within the structural limits unchanged.
use crate::report::{CheckReport, FileReport};
use pretender::config::{is_lease_expired, Mode};
use pretender::history;
use std::collections::HashSet;
use std::process::ExitCode;

use super::unit::metric_to_rule_key;
use crate::report::ViolationReport;

/// Gate baseline location: next to pretender.toml (repo root), so it can be
/// committed and gate mode behaves identically in CI.
pub(super) const BASELINE_PATH: &str = "pretender-baseline.json";

/// Single source of truth for the advisory-lease fail-closed condition
/// (GH #35): tiered/guidance with a missing or expired lease fail closed;
/// gate never consults the lease.
pub(super) fn lease_fail_closed(mode: Mode, advisory_until: Option<&str>) -> bool {
    mode.is_advisory() && (advisory_until.is_none() || is_lease_expired(advisory_until))
}

/// Human-readable remediation for a lease fail-closed (GH #35). Printed to
/// stderr and surfaced as a structured envelope warning.
pub(super) fn lease_diagnostic(mode: Mode) -> String {
    let mode_name = match mode {
        Mode::Guidance => "guidance",
        Mode::Tiered => "tiered",
        Mode::Gate => "gate",
    };
    format!(
        "mode={mode_name} requires a current advisory lease: set advisory_until in pretender.toml, \
         pass --advisory-until YYYY-MM-DD, or switch to mode=gate"
    )
}

/// The gate verdict for a completed scan: failure on any lease violation or
/// gating finding in gate mode; advisory modes always succeed.
pub(super) fn decide_exit_code(
    report: &CheckReport,
    mode: Mode,
    advisory_until: Option<&str>,
    baselined: &HashSet<String>,
) -> ExitCode {
    // Advisory lease TTL (pretender-1te): tiered/guidance are dated
    // downgrades from the gate default. Expired/missing/unparseable lease
    // fails closed — the drift detector forcing a renewed, dated decision.
    if lease_fail_closed(mode, advisory_until) {
        return ExitCode::FAILURE;
    }
    let gate_blocks = has_gating_finding(report, baselined);
    match mode {
        Mode::Gate if gate_blocks => ExitCode::FAILURE,
        Mode::Guidance | Mode::Tiered | Mode::Gate => ExitCode::SUCCESS,
    }
}

/// Whether the scan holds any finding that gates in gate mode.
fn has_gating_finding(report: &CheckReport, baselined: &HashSet<String>) -> bool {
    let has_skipped = report
        .files
        .iter()
        .any(|file| file.diagnostics.iter().any(|d| d.severity == "Error"));
    // gently-if4: a violation gates only when its stable finding ID is NOT in
    // the adopted baseline (genuinely new). Baselined violations pass; the
    // other gate categories (parse errors, coupling, cycles, test durations)
    // have no stable per-violation IDs and always gate — fail-closed.
    let has_violation = violation_findings(report)
        .iter()
        .any(|finding| !baselined.contains(&violation_finding_id(finding)));
    let has_coupling_violation = report
        .modules
        .iter()
        .any(|m| !m.coupling_violations.is_empty());
    let has_cycle = !report.cycles.is_empty();
    let has_duration_violation = !report.test_findings.is_empty();
    has_violation || has_skipped || has_coupling_violation || has_cycle || has_duration_violation
}

/// A metric violation as a stable finding: everything needed to derive the
/// u8a finding ID (`history::finding_id`) plus the event fields. File-scope
/// violations (file_lines, mock_count, ...) have `unit_name: None` and derive
/// their ID with an empty unit name: `path::::rule_key`.
pub(crate) struct ViolationFinding {
    pub(crate) path: String,
    pub(crate) unit_name: Option<String>,
    pub(crate) role: String,
    pub(crate) metric: String,
    pub(crate) rule_key: String,
    pub(crate) actual: f64,
    pub(crate) limit: f64,
}

/// Every metric violation in the report as a stable finding — unit-scope and
/// file-scope alike. The single source both history events and the gate
/// baseline derive their IDs from, so the two can never drift.
pub(crate) fn violation_findings(report: &CheckReport) -> Vec<ViolationFinding> {
    let mut findings = Vec::new();
    for file in &report.files {
        collect_file_findings(&mut findings, file);
    }
    findings
}

/// One shared finding shape for file-level and unit-level violations.
fn finding(
    path: &str,
    unit_name: Option<String>,
    file: &FileReport,
    violation: &ViolationReport,
) -> ViolationFinding {
    ViolationFinding {
        path: path.to_string(),
        unit_name,
        role: file.role.clone(),
        metric: violation.metric.clone(),
        rule_key: metric_to_rule_key(&violation.metric).to_string(),
        actual: violation.actual,
        limit: violation.limit,
    }
}

/// Collect one file's file-scope and unit-scope findings. The leading "./"
/// is normalized away so fingerprints are stable regardless of how the CLI
/// path was spelled (e.g. "src/foo.rs" vs "./src/foo.rs").
fn collect_file_findings(out: &mut Vec<ViolationFinding>, file: &FileReport) {
    let path = file
        .path
        .strip_prefix("./")
        .unwrap_or(&file.path)
        .to_string();
    for violation in &file.file_violations {
        out.push(finding(&path, None, file, violation));
    }
    for unit in &file.units {
        for violation in &unit.violations {
            out.push(finding(&path, Some(unit.name.clone()), file, violation));
        }
    }
}

pub(crate) fn violation_finding_id(finding: &ViolationFinding) -> String {
    history::finding_id(
        &finding.path,
        finding.unit_name.as_deref().unwrap_or(""),
        &finding.rule_key,
    )
}

/// The current scan's stable finding IDs — what the gate baseline is adopted
/// from and compared against.
pub(crate) fn violation_finding_ids(report: &CheckReport) -> Vec<String> {
    violation_findings(report)
        .iter()
        .map(violation_finding_id)
        .collect()
}

#[cfg(test)]
mod tests;
