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
use crate::{CheckReport, FileReport, metric_to_rule_key};
use pretender::config::{Mode, is_lease_expired};
use pretender::history;
use std::collections::HashSet;
use std::process::ExitCode;

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
        Mode::Guidance => ExitCode::SUCCESS,
        Mode::Tiered => ExitCode::SUCCESS,
        Mode::Gate => {
            if gate_blocks {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
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

    has_violation
        || has_skipped
        || has_coupling_violation
        || has_cycle
        || has_duration_violation
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

/// Collect one file's file-scope and unit-scope findings. The leading "./"
/// is normalized away so fingerprints are stable regardless of how the CLI
/// path was spelled (e.g. "src/foo.rs" vs "./src/foo.rs").
fn collect_file_findings(out: &mut Vec<ViolationFinding>, file: &FileReport) {
    let path = file.path.strip_prefix("./").unwrap_or(&file.path).to_string();
    for violation in &file.file_violations {
        out.push(ViolationFinding {
            path: path.clone(),
            unit_name: None,
            role: file.role.clone(),
            metric: violation.metric.clone(),
            rule_key: metric_to_rule_key(&violation.metric).to_string(),
            actual: violation.actual,
            limit: violation.limit,
        });
    }
    for unit in &file.units {
        for violation in &unit.violations {
            out.push(ViolationFinding {
                path: path.clone(),
                unit_name: Some(unit.name.clone()),
                role: file.role.clone(),
                metric: violation.metric.clone(),
                rule_key: metric_to_rule_key(&violation.metric).to_string(),
                actual: violation.actual,
                limit: violation.limit,
            });
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

#[test]
fn test_lease_fail_closed_matrix() {
    // advisory modes: missing or expired lease fails closed
    assert!(lease_fail_closed(Mode::Tiered, None));
    assert!(lease_fail_closed(Mode::Guidance, None));
    assert!(lease_fail_closed(Mode::Tiered, Some("2000-01-01")));
    assert!(lease_fail_closed(Mode::Guidance, Some("not-a-date")));
    // valid lease passes
    assert!(!lease_fail_closed(Mode::Tiered, Some("2099-12-31")));
    // gate never consults the lease (GH #35 regression guard)
    assert!(!lease_fail_closed(Mode::Gate, None));
    assert!(!lease_fail_closed(Mode::Gate, Some("2000-01-01")));
}

#[test]
fn test_lease_diagnostic_mentions_all_remediations() {
    let msg = lease_diagnostic(Mode::Tiered);
    assert!(msg.contains("tiered"), "names the mode: {msg}");
    assert!(msg.contains("advisory_until"), "config remediation: {msg}");
    assert!(msg.contains("--advisory-until"), "CLI remediation: {msg}");
    assert!(msg.contains("mode=gate"), "gate fallback: {msg}");
}

#[cfg(test)]
mod baseline_gate_tests {
    use super::*;
    use crate::{
        CouplingViolationReport, FileReport, MetricValues, ModuleReport, UnitReport,
        ViolationReport, violation_finding_ids,
    };

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

    fn unit_violation(metric: &str, actual: f64, limit: f64) -> ViolationReport {
        ViolationReport {
            metric: metric.to_string(),
            actual,
            limit,
            line: None,
        }
    }

    fn file_report(
        path: &str,
        file_violations: Vec<ViolationReport>,
        units: Vec<UnitReport>,
    ) -> FileReport {
        FileReport {
            path: path.to_string(),
            role: "app".to_string(),
            diagnostics: vec![],
            file_violations,
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

    fn baselined_ids(r: &CheckReport) -> HashSet<String> {
        violation_finding_ids(r).into_iter().collect()
    }

    /// A mechanical reformat shifted lines: same file, same function name,
    /// same rule — only start_line differs (50 -> 77).
    fn shifted_violation_report() -> CheckReport {
        let mut r = report(vec![file_report(
            "src/a.rs",
            vec![unit_violation("file_lines", 420.0, 400.0)],
            vec![unit_report(
                "parse",
                vec![unit_violation("function_lines", 120.0, 100.0)],
            )],
        )]);
        r.files[0].units[0].start_line = 77;
        r
    }

    #[test]
    fn gate_fails_on_any_violation_without_baseline() {
        // No baseline file: fail-closed, gate fails on any violation.
        let r = shifted_violation_report();
        assert_eq!(
            decide_exit_code(&r, Mode::Gate, None, &HashSet::new()),
            ExitCode::FAILURE
        );
    }

    #[test]
    fn gate_passes_when_only_baselined_violations_remain() {
        // gently-if4 acceptance: "0 fixed, 11 still open, 0 new" must exit 0.
        let r = shifted_violation_report();
        let baselined = baselined_ids(&r);
        assert_eq!(baselined.len(), 2, "file + unit finding");
        assert_eq!(
            decide_exit_code(&r, Mode::Gate, None, &baselined),
            ExitCode::SUCCESS
        );
    }

    #[test]
    fn gate_fails_on_genuinely_new_violation_with_baseline() {
        let adopted = shifted_violation_report();
        let baselined = baselined_ids(&adopted);
        // A different function now violates too — genuinely new.
        let mut r = adopted;
        r.files[0].units.push(unit_report(
            "serialize",
            vec![unit_violation("cognitive", 30.0, 15.0)],
        ));
        assert_eq!(
            decide_exit_code(&r, Mode::Gate, None, &baselined),
            ExitCode::FAILURE
        );
    }

    #[test]
    fn gate_fails_on_coupling_even_when_metric_violations_baselined() {
        // Fail-closed preserved: baseline covers metric violations only —
        // coupling violations, cycles and test-duration findings always gate.
        let mut r = shifted_violation_report();
        r.modules.push(ModuleReport {
            path: "src/a.rs".to_string(),
            coupling_violations: vec![CouplingViolationReport {
                metric: "affero".to_string(),
                actual: 9.0,
                limit: 5.0,
            }],
        });
        let baselined = baselined_ids(&r);
        assert_eq!(
            decide_exit_code(&r, Mode::Gate, None, &baselined),
            ExitCode::FAILURE
        );
    }

    #[test]
    fn finding_ids_reuse_history_scheme_file_and_unit_scope() {
        // One ID scheme (u8a finding_id), not a second one: file scope uses
        // an empty unit name; no line numbers anywhere.
        let r = shifted_violation_report();
        let mut ids = violation_finding_ids(&r);
        ids.sort();
        let mut expected = vec![
            "src/a.rs::parse::function_lines_max".to_string(),
            "src/a.rs::::file_lines".to_string(),
        ];
        expected.sort();
        assert_eq!(ids, expected);
    }

    #[test]
    fn finding_ids_stable_across_line_shifts() {
        let before = shifted_violation_report();
        let mut after = before.clone();
        after.files[0].units[0].start_line = 4096;
        assert_eq!(
            violation_finding_ids(&before),
            violation_finding_ids(&after),
            "a mechanical reformat must not change finding IDs"
        );
    }

    #[test]
    fn adopt_writes_baseline_that_gate_accepts() {
        // The --adopt flow: persist the current scan's IDs, reload, and gate
        // mode passes on exactly those violations.
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("pretender-baseline.json");
        let r = shifted_violation_report();
        let ids = violation_finding_ids(&r);
        history::persist_baseline(&path, &ids).expect("persist");
        let loaded = history::load_baseline(&path)
            .expect("load")
            .expect("exists");
        let baselined: HashSet<String> = loaded.finding_ids.into_iter().collect();
        assert_eq!(
            decide_exit_code(&r, Mode::Gate, None, &baselined),
            ExitCode::SUCCESS
        );
    }
}
