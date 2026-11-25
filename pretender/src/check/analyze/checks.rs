//! File-level check stages: one function per check, sharing
//! `super::FileCheckCtx`.
//!
//! # Responsibilities
//! Every per-file violation except the file-size limit (applied in
//! `super::run_file_checks`): mock overuse, void mutators, mutable-state
//! ratio, unwrap density, primitive obsession, data model, inheritance
//! depth, and lazy test clusters — in the original evaluation order.
//!
//! # Rationale
//! Split from the per-file analysis module (pretender-5zo.15.12) along the
//! pipeline/check-stages seam; bodies moved verbatim, with the two long
//! stage functions split into verbatim-behaving helpers.
use super::{FileCheckCtx, FileFacts};
use crate::report::ViolationReport;
use pretender::roles::Role;
use pretender::{
    cluster_detector, data_model, error_metrics, inheritance_depth, mock_detector,
    mutability_metrics, primitive_obsession,
};

/// A file-scope violation with no source line attached.
fn violation(metric: &str, actual: f64, limit: f64) -> ViolationReport {
    ViolationReport {
        metric: metric.to_string(),
        actual,
        limit,
        line: None,
    }
}

/// Mock-overuse detection for test-role files.
pub(super) fn check_mock_overuse(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts, role: Role) {
    if role == Role::Test && ctx.config.thresholds.test.checks.mock_count_max > 0 {
        let analysis = mock_detector::detect_mocks(
            ctx.source,
            &ctx.module.language,
            &ctx.config.patterns.mock.extra,
            ctx.path,
        );
        if analysis.total_usage > ctx.config.thresholds.test.checks.mock_count_max {
            let max = ctx.config.thresholds.test.checks.mock_count_max;
            facts.file_violations.push(violation(
                "mock_count",
                analysis.total_usage as f64,
                max as f64,
            ));
        }
        facts.mock_analysis = analysis;
    }
}

/// Void-mutator detection (non-test roles by default, role-specific
/// threshold).
pub(super) fn check_void_mutators(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts) {
    if ctx.thresholds.file.void_mutators_max > 0 {
        let mutators = mutability_metrics::detect_void_mutators(
            ctx.source,
            &ctx.module.language,
            &ctx.module.units,
            ctx.path,
        );
        if mutators.len() as u32 > ctx.thresholds.file.void_mutators_max {
            let max = ctx.thresholds.file.void_mutators_max;
            facts.file_violations.push(violation(
                "void_mutator",
                mutators.len() as f64,
                max as f64,
            ));
        }
    }
}

/// Mutable-state ratio check.
pub(super) fn check_mut_ratio(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts) {
    let max = ctx.config.thresholds.app.checks.mut_ratio_max;
    if max > 0.0 {
        let binding_analysis =
            mutability_metrics::count_mutable_bindings(ctx.source, &ctx.module.language);
        let ratio = binding_analysis.ratio();
        if ratio > max {
            facts
                .file_violations
                .push(violation("mut_ratio", ratio, max));
        }
    }
}

/// Unwrap-density check.
pub(super) fn check_unwrap_density(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts) {
    if ctx.thresholds.file.unwrap_max > 0 {
        let sites = error_metrics::detect_unwraps(ctx.source, &ctx.module.language);
        if sites.len() as u32 > ctx.thresholds.file.unwrap_max {
            let max = ctx.thresholds.file.unwrap_max;
            facts
                .file_violations
                .push(violation("unwraps", sites.len() as f64, max as f64));
        }
    }
}

/// Primitive-obsession check: bool clusters and primitive domain params.
pub(super) fn check_primitive_obsession(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts) {
    if ctx.config.thresholds.app.checks.bool_cluster_max > 0
        || ctx.config.thresholds.app.checks.primitive_param_check
    {
        let po_results = primitive_obsession::analyze(
            &ctx.module.units,
            ctx.config.thresholds.app.checks.bool_cluster_max,
            ctx.config.thresholds.app.checks.primitive_param_check,
            &[],
        );
        for result in &po_results {
            check_po_result(ctx, facts, result);
        }
    }
}

/// One primitive-obsession result: bool-cluster violation plus its
/// primitive-domain-param violations.
fn check_po_result(
    ctx: &FileCheckCtx<'_>,
    facts: &mut FileFacts,
    result: &primitive_obsession::PrimitiveObsessionAnalysis,
) {
    check_bool_cluster(ctx, facts, result.bool_param_count);
    for v in &result.violations {
        if v.kind == primitive_obsession::PrimitiveViolationKind::PrimitiveDomainParam {
            facts
                .file_violations
                .push(violation("primitive_param", 1.0, 1.0));
        }
    }
}

/// The bool-cluster violation when a unit's bool parameter count meets the
/// configured cluster size.
fn check_bool_cluster(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts, bool_count: u32) {
    let max = ctx.config.thresholds.app.checks.bool_cluster_max;
    if bool_count > 0 && max > 0 && bool_count >= max {
        facts
            .file_violations
            .push(violation("bool_cluster", bool_count as f64, max as f64));
    }
}

/// Data-model checks (type definitions from the universal model).
/// excessive_fields is role-overridable (effective thresholds);
/// primitive_field/domain_patterns are app-level only (spec: not
/// role-mergeable).
pub(super) fn check_data_model(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts) {
    if ctx.thresholds.file.excessive_fields_max > 0
        || ctx.config.thresholds.app.checks.primitive_field_check
    {
        let findings = data_model::analyze(
            &ctx.module.type_definitions,
            ctx.thresholds.file.excessive_fields_max,
            ctx.config.thresholds.app.checks.primitive_field_check,
            &ctx.config.thresholds.app.checks.domain_patterns,
        );
        for finding in findings {
            facts.file_violations.push(ViolationReport {
                metric: finding.metric.to_string(),
                actual: finding.actual as f64,
                limit: finding.limit as f64,
                line: Some(finding.span.start_line),
            });
        }
    }
}

/// Inheritance depth check.
pub(super) fn check_inheritance_depth(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts) {
    let max = ctx.config.thresholds.app.checks.inheritance_depth_max;
    if max == 0 {
        return;
    }
    let ih = inheritance_depth::analyze(ctx.source, &ctx.module.language);
    for class in ih.classes.iter().filter(|class| class.depth > max) {
        facts.file_violations.push(violation(
            "inheritance_depth",
            class.depth as f64,
            max as f64,
        ));
    }
}

/// Lazy test cluster detection for test-role files.
pub(super) fn check_lazy_clusters(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts, role: Role) {
    if role == Role::Test && ctx.config.thresholds.test.checks.lazy_cluster_min > 0 {
        let clusters = cluster_detector::detect(
            &ctx.module.units,
            ctx.config.thresholds.test.checks.lazy_cluster_min,
        );
        for cluster in &clusters {
            let min = ctx.config.thresholds.test.checks.lazy_cluster_min;
            facts
                .file_violations
                .push(violation("lazy_cluster", cluster.count as f64, min as f64));
        }
    }
}
