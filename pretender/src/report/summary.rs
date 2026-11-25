//! Severity/band summarization shared by the report writers.
//!
//! Derives the worst-severity message shown for a file from its
//! diagnostics, file violations, and per-unit banded metrics, and owns the
//! metric formatting used in human-readable output.
//!
//! # Responsibilities
//! Map metric observations to severities and the single message a file's
//! summary line shows; format metric values for display.
//!
//! # Rationale
//! Split out of the extracted report writers (epic pretender-5zo) so the
//! human, markdown, and HTML modules share one summarization seam;
//! `summarize_file` is a consuming-builder chain over stage helpers so
//! every function stays within the gate's limits and the file stays
//! mutation-free; bodies moved verbatim from `main.rs`.

use super::{FileReport, UnitReport};
use pretender::config::{Band, Bands};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Severity {
    Green,
    Yellow,
    Red,
}

pub(super) struct FileSummary {
    pub(super) severity: Severity,
    pub(super) message: String,
}

pub(super) fn summarize_file(file: &FileReport, bands: &Bands) -> FileSummary {
    FileSummary {
        severity: Severity::Green,
        message: "all green".to_string(),
    }
    .with_diagnostics(file)
    .with_file_violations(file)
    .with_units(file, bands)
}

impl FileSummary {
    /// Considers `file`'s parse errors, in file order.
    fn with_diagnostics(mut self, file: &FileReport) -> Self {
        for diagnostic in &file.diagnostics {
            if diagnostic.severity == "Error" {
                consider_summary(
                    &mut self,
                    Severity::Red,
                    format!("parse error: {}", diagnostic.message),
                );
            }
        }
        self
    }

    /// Considers `file`'s file-level violations, in file order.
    fn with_file_violations(mut self, file: &FileReport) -> Self {
        for violation in &file.file_violations {
            consider_summary(
                &mut self,
                Severity::Red,
                format!(
                    "{} {:.0} > {:.0}",
                    violation.metric, violation.actual, violation.limit
                ),
            );
        }
        self
    }

    /// Considers `file`'s per-unit banded metrics and violations, in order.
    fn with_units(mut self, file: &FileReport, bands: &Bands) -> Self {
        for unit in &file.units {
            summarize_unit_bands(unit, bands, &mut self);
            summarize_unit_violations(unit, &mut self);
        }
        self
    }
}

fn summarize_unit_bands(unit: &UnitReport, bands: &Bands, summary: &mut FileSummary) {
    consider_banded_metric(
        summary,
        &unit.name,
        MetricHit {
            metric: "cyclomatic",
            actual: unit.metrics.cyclomatic,
            band: bands.cyclomatic,
        },
    );
    consider_banded_metric(
        summary,
        &unit.name,
        MetricHit {
            metric: "cognitive",
            actual: unit.metrics.cognitive,
            band: bands.cognitive,
        },
    );
}

fn summarize_unit_violations(unit: &UnitReport, summary: &mut FileSummary) {
    for violation in &unit.violations {
        if violation.metric == "cyclomatic" || violation.metric == "cognitive" {
            continue;
        }
        consider_summary(
            summary,
            Severity::Red,
            format!(
                "{}(): {} {:.0} > {:.0}",
                unit.name, violation.metric, violation.actual, violation.limit
            ),
        );
    }
}

/// One banded metric observation to consider for a file summary.
struct MetricHit {
    metric: &'static str,
    actual: u32,
    band: Option<Band>,
}

fn consider_banded_metric(summary: &mut FileSummary, unit_name: &str, hit: MetricHit) {
    let MetricHit {
        metric,
        actual,
        band,
    } = hit;
    let severity = band_severity(actual, band);
    if severity == Severity::Green {
        return;
    }

    consider_summary(
        summary,
        severity,
        banded_message(unit_name, metric, actual, severity),
    );
}

/// The summary message for a banded metric, e.g. `fn(): cyclomatic 12 (red)`.
fn banded_message(unit_name: &str, metric: &str, actual: u32, severity: Severity) -> String {
    format!(
        "{}(): {} {} ({})",
        unit_name,
        metric,
        actual,
        severity_label(severity)
    )
}

fn consider_summary(summary: &mut FileSummary, severity: Severity, message: String) {
    if severity > summary.severity {
        summary.severity = severity;
        summary.message = message;
    }
}

fn band_severity(actual: u32, band: Option<Band>) -> Severity {
    match band {
        Some(band) if actual > band.yellow => Severity::Red,
        Some(band) if actual > band.green => Severity::Yellow,
        _ => Severity::Green,
    }
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Green => "green",
        Severity::Yellow => "yellow",
        Severity::Red => "red",
    }
}

pub(super) fn fmt_metric(v: f64) -> String {
    if v == v.floor() {
        format!("{:.0}", v)
    } else {
        format!("{:.2}", v)
    }
}
