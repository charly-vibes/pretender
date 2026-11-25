//! Lease-matrix and gate-baseline tests for `super` (the check gate).
//!
//! Moved verbatim from the single-file gate.rs (pretender-5zo.15.12) so the
//! gate implementation stays within the per-file size limit; the
//! fail-closed lease matrix and the adopt/baseline gate behavior they pin
//! are unchanged.

use super::*;

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
    use crate::report::{
        CouplingViolationReport, FileReport, MetricValues, ModuleReport, UnitReport,
        ViolationReport,
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
