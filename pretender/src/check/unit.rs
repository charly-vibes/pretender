//! Unit-level metric violations: build the per-unit report.
//!
//! Computes the metric values for one code unit and scores it against the
//! role-merged limits, including exported-only and test-assertion minimums.
//!
//! # Responsibilities
//! `build_unit_report` is the single unit scorer; the `*_violation`
//! constructors are the shared violation builders used by both unit and
//! file-level checks.
//!
//! # Rationale
//! Extracted verbatim from main.rs (pretender-5zo.2.2); the original
//! 82-line `build_unit_report` was split into stage helpers while moving
//! so violation order and semantics stay identical within the structural
//! limits. The push-into-`&mut` accumulator style was restructured into
//! Option-returning constructors chained in evaluation order (same
//! violations, same order, no mutable bindings).
use crate::report::{MetricValues, UnitReport, ViolationReport};
use pretender::metrics;
use pretender::model::CodeUnit;
use pretender::roles::EffectiveThresholds;

/// Ledger/rule-key name for a raw violation metric (`_max` suffix except
/// where the metric is already a key) — the stable finding-ID vocabulary.
pub(super) fn metric_to_rule_key(metric: &str) -> &str {
    match metric {
        "cognitive" => "cognitive_max",
        "cyclomatic" => "cyclomatic_max",
        "params" => "params_max",
        "nesting" => "nesting_max",
        "function_lines" => "function_lines_max",
        "abc" => "abc_max",
        "duplication_pct" => "duplication_pct_max",
        "min_assertions" => "min_assertions",
        other => other,
    }
}

/// Compute and score one code unit against role-merged thresholds.
pub(super) fn build_unit_report(unit: &CodeUnit, thresholds: &EffectiveThresholds) -> UnitReport {
    let metrics = unit_metrics(unit);
    let violations = unit_violations(unit, &metrics, thresholds);
    UnitReport {
        name: unit.name.clone(),
        kind: format!("{:?}", unit.kind),
        start_line: unit.span.start_line,
        metrics,
        violations,
    }
}

/// Every violation for one unit, in the original evaluation order: the five
/// base limits plus abc, then the test-assertion minimum, then the
/// exported-only limits.
fn unit_violations(
    unit: &CodeUnit,
    metrics: &MetricValues,
    thresholds: &EffectiveThresholds,
) -> Vec<ViolationReport> {
    base_limit_violations(metrics, thresholds)
        .into_iter()
        .chain(min_assertion_violation(unit, metrics, thresholds))
        .chain(exported_violations(unit, metrics, thresholds))
        .collect()
}

/// Metric values for one unit, straight from the shared metrics module.
fn unit_metrics(unit: &CodeUnit) -> MetricValues {
    MetricValues {
        cyclomatic: metrics::cyclomatic(unit),
        cognitive: metrics::cognitive(unit),
        assertions: unit.assertions,
        function_lines: metrics::function_lines(unit),
        params: metrics::params(unit),
        nesting_max: metrics::nesting_max(unit),
        abc: metrics::abc(unit),
    }
}

/// Score the five base limits plus abc against the metrics.
fn base_limit_violations(
    metrics: &MetricValues,
    thresholds: &EffectiveThresholds,
) -> Vec<ViolationReport> {
    // Single-letter locals keep every table entry on one fmt-stable line.
    let (m, f) = (metrics, &thresholds.function);
    let limits = [
        ("cyclomatic", m.cyclomatic, f.cyclomatic_max),
        ("cognitive", m.cognitive, f.cognitive_max),
        ("function_lines", m.function_lines, f.function_lines_max),
        ("params", m.params, f.params_max),
        ("nesting_max", m.nesting_max, f.nesting_max),
    ];
    limits
        .into_iter()
        .filter_map(|(name, actual, max)| limit_violation(name, actual, max))
        .chain(limit_violation_f64("abc", m.abc, f.abc_max))
        .collect()
}

/// The test-assertion minimum, applied only to units with test identity —
/// a test-registration attribute or a test-shaped name (pretender-68j).
/// Fixtures and helpers living in test-role files are exempt.
fn min_assertion_violation(
    unit: &CodeUnit,
    metrics: &MetricValues,
    thresholds: &EffectiveThresholds,
) -> Vec<ViolationReport> {
    let Some(min) = thresholds.min_assertions else {
        return Vec::new();
    };
    if !(unit.has_test_attribute || has_test_name(&unit.name)) {
        return Vec::new();
    }
    min_violation("min_assertions", metrics.assertions, min)
        .into_iter()
        .collect()
}

/// The exported-only limits for exported units, in the original order.
fn exported_violations(
    unit: &CodeUnit,
    metrics: &MetricValues,
    thresholds: &EffectiveThresholds,
) -> Vec<ViolationReport> {
    if !unit.is_exported {
        return Vec::new();
    }
    let (m, e) = (metrics, &thresholds.exports);
    let limits = [
        (m.cyclomatic, e.exported_cyclomatic_max),
        (m.params, e.exported_params_max),
        (m.function_lines, e.exported_lines_max),
    ];
    limits
        .into_iter()
        .zip(["exported_cyclomatic", "exported_params", "exported_lines"])
        .filter_map(|((actual, max), name)| exported_violation(name, actual, max))
        .collect()
}

/// An exported-only violation when `max` is set and `actual` exceeds it.
fn exported_violation(
    metric: &'static str,
    actual: u32,
    max: Option<u32>,
) -> Option<ViolationReport> {
    max.and_then(|max| limit_violation(metric, actual, max))
}

/// A violation when `actual` exceeds `max`.
pub(super) fn limit_violation(
    metric: &'static str,
    actual: u32,
    max: u32,
) -> Option<ViolationReport> {
    (actual > max).then(|| ViolationReport {
        metric: metric.to_string(),
        actual: actual as f64,
        limit: max as f64,
        line: None,
    })
}

/// A violation when a floating-point `actual` exceeds `max`.
fn limit_violation_f64(metric: &'static str, actual: f64, max: u32) -> Option<ViolationReport> {
    (actual > max as f64).then(|| ViolationReport {
        metric: metric.to_string(),
        actual,
        limit: max as f64,
        line: None,
    })
}

/// A violation when `actual` falls below a minimum.
fn min_violation(metric: &'static str, actual: u32, min: u32) -> Option<ViolationReport> {
    (actual < min).then(|| ViolationReport {
        metric: metric.to_string(),
        actual: actual as f64,
        limit: min as f64,
        line: None,
    })
}

/// Test-shaped unit names, per the update-min-assertions-scope proposal:
/// `^test_`, `_test$`, `^test[A-Z]`, `Test$`, `^test$`.
fn has_test_name(name: &str) -> bool {
    name == "test"
        || name.starts_with("test_")
        || name.ends_with("_test")
        || (name.len() > 4 && name.starts_with("test") && name.as_bytes()[4].is_ascii_uppercase())
        || (name.len() >= 4 && name.ends_with("Test"))
}
