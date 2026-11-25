//! SARIF check-report writer.
//!
//! Renders `CheckReport` as a SARIF 2.1.0 log: one rule per violated
//! metric, one result per violation, plus test-duration findings.
//!
//! # Responsibilities
//! Emit the SARIF format byte-identically to the pre-extraction
//! implementation; accumulate rules/results in report order.
//!
//! # Rationale
//! Split out of the extracted report writers (epic pretender-5zo); the
//! former 7-parameter closure became a `SarifAccumulator` with stage
//! helpers so every function fits the gate's limits; bodies moved verbatim
//! from `main.rs`.

use super::{CheckReport, ViolationReport};
use anyhow::Result;
use serde_sarif::sarif;
use std::collections::HashMap;
use std::io::Write;

/// Accumulates SARIF rules and results while walking a `CheckReport`.
#[derive(Default)]
struct SarifAccumulator {
    rule_index: HashMap<String, i64>,
    rules: Vec<sarif::ReportingDescriptor>,
    results: Vec<sarif::Result>,
}

/// A violation to record, with its SARIF location context.
struct ViolationContext<'a> {
    violation: &'a ViolationReport,
    file_path: &'a str,
    start_line: i64,
    label: &'a str,
}

impl SarifAccumulator {
    /// Index of `violation.metric`'s rule, registering the rule on first use.
    fn rule_index_for(&mut self, violation: &ViolationReport) -> i64 {
        if let Some(&i) = self.rule_index.get(&violation.metric) {
            return i;
        }
        let i = self.rules.len() as i64;
        self.rules.push(
            sarif::ReportingDescriptor::builder()
                .id(&violation.metric)
                .build(),
        );
        self.rule_index.insert(violation.metric.clone(), i);
        i
    }

    /// Records one violation against its rule at its location context.
    fn push_result(&mut self, ctx: &ViolationContext<'_>) {
        let idx = self.rule_index_for(ctx.violation);
        let message_text = format!(
            "{} in {} exceeds limit: actual={:.0}, limit={:.0}",
            ctx.violation.metric, ctx.label, ctx.violation.actual, ctx.violation.limit
        );
        let location = violation_location(ctx.violation, ctx.file_path, ctx.start_line);
        self.results.push(sarif_result(
            &ctx.violation.metric,
            idx,
            message_text,
            location,
        ));
    }

    /// File and unit violations of `report`, in report order.
    fn collect_violations(&mut self, report: &CheckReport) {
        for file in &report.files {
            for violation in &file.file_violations {
                self.push_result(&ViolationContext {
                    violation,
                    file_path: &file.path,
                    start_line: 1,
                    label: &file.path,
                });
            }
            for unit in &file.units {
                for violation in &unit.violations {
                    self.push_result(&ViolationContext {
                        violation,
                        file_path: &file.path,
                        start_line: unit.start_line as i64,
                        label: &unit.name,
                    });
                }
            }
        }
    }

    /// Test-duration findings, in report order.
    fn collect_duration_findings(&mut self, report: &CheckReport) {
        // ── SARIF: duration findings ───────────────────────────────
        let duration_rule_id = "pretender/test-duration";
        let duration_rule_idx = self.rules.len() as i64;
        self.rules.push(
            sarif::ReportingDescriptor::builder()
                .id(duration_rule_id)
                .build(),
        );
        for finding in &report.test_findings {
            let file_uri = finding
                .file
                .as_ref()
                .map(|f| f.display().to_string())
                .unwrap_or_else(|| "?".to_string());
            let message_text = format!(
                "{} {}: {}ms exceeds {}ms threshold",
                finding.role, finding.test_name, finding.observed_ms, finding.threshold_ms
            );
            let location = duration_location(&file_uri);
            self.results.push(sarif_result(
                duration_rule_id,
                duration_rule_idx,
                message_text,
                location,
            ));
        }
    }

    /// Assembles the finished SARIF log.
    fn build_log(self) -> sarif::Sarif {
        let tool_component = sarif::ToolComponent::builder()
            .name("pretender")
            .rules(self.rules)
            .build();
        let run = sarif::Run::builder()
            .tool(tool_component)
            .results(self.results)
            .build();
        sarif::Sarif::builder()
            .version(sarif::Version::V2_1_0.to_string())
            .schema(sarif::SCHEMA_URL)
            .runs(vec![run])
            .build()
    }
}

/// Physical location for a violation result.
fn violation_location(
    violation: &ViolationReport,
    file_path: &str,
    start_line: i64,
) -> sarif::Location {
    sarif::Location::builder()
        .physical_location(
            sarif::PhysicalLocation::builder()
                .artifact_location(sarif::ArtifactLocation::builder().uri(file_path).build())
                .region(
                    sarif::Region::builder()
                        .start_line(violation.line.map(i64::from).unwrap_or(start_line))
                        .build(),
                )
                .build(),
        )
        .build()
}

/// Physical location for a test-duration finding.
fn duration_location(file_uri: &str) -> sarif::Location {
    sarif::Location::builder()
        .physical_location(
            sarif::PhysicalLocation::builder()
                .artifact_location(sarif::ArtifactLocation::builder().uri(file_uri).build())
                .build(),
        )
        .build()
}

/// One SARIF result referencing a rule by id and index.
fn sarif_result(
    rule_id: &str,
    rule_index: i64,
    message_text: String,
    location: sarif::Location,
) -> sarif::Result {
    sarif::Result::builder()
        .rule_id(rule_id)
        .rule_index(rule_index)
        .message(sarif::Message::builder().text(message_text).build())
        .locations(vec![location])
        .level(sarif::ResultLevel::Warning)
        .build()
}

pub(crate) fn write_sarif_report(sink: &mut dyn Write, report: &CheckReport) -> Result<()> {
    let mut acc = SarifAccumulator::default();
    acc.collect_violations(report);
    acc.collect_duration_findings(report);
    serde_json::to_writer_pretty(&mut *sink, &acc.build_log())?;
    writeln!(sink)?;
    Ok(())
}

#[cfg(test)]
mod sarif_region_tests {
    use super::super::violation;
    use super::super::{FileReport, MetricValues, UnitReport};
    use super::*;

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

    fn file_report(path: &str, units: Vec<UnitReport>) -> FileReport {
        FileReport {
            path: path.to_string(),
            role: "app".to_string(),
            diagnostics: vec![],
            file_violations: vec![],
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

    #[test]
    fn sarif_region_uses_line_when_present() {
        let file = file_report(
            "src/a.rs",
            vec![unit_report("fn_a", vec![violation(Some(50))])],
        );
        let report = report(vec![file]);
        let mut buf = Vec::new();
        write_sarif_report(&mut buf, &report).expect("sarif");
        let v: serde_json::Value = serde_json::from_slice(&buf).expect("parse sarif");
        let region = &v["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 50);
    }

    #[test]
    fn sarif_region_falls_back_to_default_start_line_when_line_none() {
        let file = file_report("src/a.rs", vec![unit_report("fn_a", vec![violation(None)])]);
        let report = report(vec![file]);
        let mut buf = Vec::new();
        write_sarif_report(&mut buf, &report).expect("sarif");
        let v: serde_json::Value = serde_json::from_slice(&buf).expect("parse sarif");
        let region = &v["runs"][0]["results"][0]["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 10);
    }
}
