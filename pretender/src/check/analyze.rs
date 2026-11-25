//! Per-file analysis: parse a source file and produce its report.
//!
//! Parses with the language-specific parser, detects the file's role, and
//! runs every file-level check (mock overuse, mutability, unwrap density,
//! primitive obsession, data model, inheritance depth, lazy clusters),
//! plus per-unit metric violations.
//!
//! # Responsibilities
//! Turn one source file into `(FileReport, Module)` or `None` when the
//! file is unreadable as UTF-8 or has no parser. Unit-level metric
//! violations live in `super::unit`; thresholds come from the role-merged
//! effective thresholds.
//!
//! # Rationale
//! Extracted verbatim from main.rs (pretender-5zo.2.2); the original
//! 203-line `analyze_path` was split into stage helpers sharing a
//! `FileCheckCtx` so every function lands within the structural limits
//! while check semantics and violation ordering stay identical.
use super::scan::get_parser;
use super::unit::{build_unit_report, limit_violation};
use crate::report::{FileReport, MockFindingReport, UnitReport, ViolationReport};
use anyhow::{Context, Result};
use pretender::config::Config;
use pretender::model::{Diagnostic, Module};
use pretender::role_resolution::RoleDetector;
use pretender::roles::{EffectiveThresholds, Role};
use pretender::{
    cluster_detector, data_model, error_metrics, inheritance_depth, mock_detector,
    mutability_metrics, primitive_obsession,
};
use std::fs;
use std::path::Path;

/// Everything a file-level check stage needs, grouped so each stage helper
/// stays within the parameter limit.
struct FileCheckCtx<'a> {
    path: &'a Path,
    source: &'a str,
    module: &'a Module,
    config: &'a Config,
    thresholds: &'a EffectiveThresholds,
}

/// Outputs accumulated across the file-level check stages.
struct FileFacts {
    units: Vec<UnitReport>,
    file_violations: Vec<ViolationReport>,
    mock_analysis: mock_detector::MockAnalysis,
}

impl FileFacts {
    /// Units scored against role thresholds; per-file violations start
    /// empty; mock analysis starts from its default detector state.
    fn for_module(module: &Module, thresholds: &EffectiveThresholds) -> FileFacts {
        FileFacts {
            units: module
                .units
                .iter()
                .map(|unit| build_unit_report(unit, thresholds))
                .collect(),
            file_violations: Vec::new(),
            mock_analysis: mock_detector::MockAnalysis::default(),
        }
    }
}

/// One parsed-and-role-detected source file, owned so the report and the
/// module can be returned together.
struct DetectedModule {
    source: String,
    module: Module,
    diagnostics: Vec<Diagnostic>,
    role: Role,
}

/// Parse and analyze one source file, returning `Ok(None)` when the file
/// should be skipped (unreadable as UTF-8, no parser for its extension).
pub(super) fn analyze_path(
    path: &Path,
    detector: &RoleDetector,
    config: &Config,
) -> Result<Option<(FileReport, Module)>> {
    let Some(dm) = detect_module(path, detector)? else {
        return Ok(None);
    };
    let thresholds = EffectiveThresholds::for_role(dm.role, &config.thresholds);
    let mut facts = FileFacts::for_module(&dm.module, &thresholds);
    let ctx = FileCheckCtx {
        path,
        source: &dm.source,
        module: &dm.module,
        config,
        thresholds: &thresholds,
    };
    run_file_checks(&ctx, &mut facts, dm.role);
    let report = assemble_file_report(&ctx, dm.role, dm.diagnostics, facts);
    Ok(Some((report, dm.module)))
}

/// Run every file-level check stage in the original evaluation order,
/// starting with the file-level line-count limit.
fn run_file_checks(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts, role: Role) {
    facts.file_violations.extend(limit_violation(
        "file_lines",
        ctx.module.lines_total,
        ctx.thresholds.file.file_lines_max,
    ));
    check_mock_overuse(ctx, facts, role);
    check_void_mutators(ctx, facts);
    check_mut_ratio(ctx, facts);
    check_unwrap_density(ctx, facts);
    check_primitive_obsession(ctx, facts);
    check_data_model(ctx, facts);
    check_inheritance_depth(ctx, facts);
    check_lazy_clusters(ctx, facts, role);
}

/// Read and parse the file, detect its role. `Ok(None)` when the source is
/// not valid UTF-8 or no parser handles the extension.
fn detect_module(path: &Path, detector: &RoleDetector) -> Result<Option<DetectedModule>> {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("failed to read source file: {}", path.display()));
        }
    };

    let parser = match get_parser(path) {
        Ok(parser) => parser,
        Err(_) => return Ok(None),
    };

    let (module, diagnostics) = parser.parse(path, &source)?;
    let role = detector.detect(path, &source);
    Ok(Some(DetectedModule {
        source,
        module,
        diagnostics,
        role,
    }))
}

/// Mock-overuse detection for test-role files.
fn check_mock_overuse(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts, role: Role) {
    if role == Role::Test && ctx.config.thresholds.test.checks.mock_count_max > 0 {
        let analysis = mock_detector::detect_mocks(
            ctx.source,
            &ctx.module.language,
            &ctx.config.patterns.mock.extra,
            ctx.path,
        );
        if analysis.total_usage > ctx.config.thresholds.test.checks.mock_count_max {
            facts.file_violations.push(ViolationReport {
                metric: "mock_count".to_string(),
                actual: analysis.total_usage as f64,
                limit: ctx.config.thresholds.test.checks.mock_count_max as f64,
                line: None,
            });
        }
        facts.mock_analysis = analysis;
    }
}

/// Void-mutator detection (non-test roles by default, role-specific
/// threshold).
fn check_void_mutators(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts) {
    if ctx.thresholds.file.void_mutators_max > 0 {
        let mutators = mutability_metrics::detect_void_mutators(
            ctx.source,
            &ctx.module.language,
            &ctx.module.units,
            ctx.path,
        );
        if mutators.len() as u32 > ctx.thresholds.file.void_mutators_max {
            facts.file_violations.push(ViolationReport {
                metric: "void_mutator".to_string(),
                actual: mutators.len() as f64,
                limit: ctx.thresholds.file.void_mutators_max as f64,
                line: None,
            });
        }
    }
}

/// Mutable-state ratio check.
fn check_mut_ratio(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts) {
    if ctx.config.thresholds.app.checks.mut_ratio_max > 0.0 {
        let binding_analysis =
            mutability_metrics::count_mutable_bindings(ctx.source, &ctx.module.language);
        let ratio = binding_analysis.ratio();
        if ratio > ctx.config.thresholds.app.checks.mut_ratio_max {
            facts.file_violations.push(ViolationReport {
                metric: "mut_ratio".to_string(),
                actual: ratio,
                limit: ctx.config.thresholds.app.checks.mut_ratio_max,
                line: None,
            });
        }
    }
}

/// Unwrap-density check.
fn check_unwrap_density(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts) {
    if ctx.thresholds.file.unwrap_max > 0 {
        let sites = error_metrics::detect_unwraps(ctx.source, &ctx.module.language);
        if sites.len() as u32 > ctx.thresholds.file.unwrap_max {
            facts.file_violations.push(ViolationReport {
                metric: "unwraps".to_string(),
                actual: sites.len() as f64,
                limit: ctx.thresholds.file.unwrap_max as f64,
                line: None,
            });
        }
    }
}

/// Primitive-obsession check: bool clusters and primitive domain params.
fn check_primitive_obsession(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts) {
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
    if result.bool_param_count > 0
        && ctx.config.thresholds.app.checks.bool_cluster_max > 0
        && result.bool_param_count >= ctx.config.thresholds.app.checks.bool_cluster_max
    {
        facts.file_violations.push(ViolationReport {
            metric: "bool_cluster".to_string(),
            actual: result.bool_param_count as f64,
            limit: ctx.config.thresholds.app.checks.bool_cluster_max as f64,
            line: None,
        });
    }
    for v in &result.violations {
        if v.kind == primitive_obsession::PrimitiveViolationKind::PrimitiveDomainParam {
            facts.file_violations.push(ViolationReport {
                metric: "primitive_param".to_string(),
                actual: 1.0,
                limit: 1.0,
                line: None,
            });
        }
    }
}

/// Data-model checks (type definitions from the universal model).
/// excessive_fields is role-overridable (effective thresholds);
/// primitive_field/domain_patterns are app-level only (spec: not
/// role-mergeable).
fn check_data_model(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts) {
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
fn check_inheritance_depth(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts) {
    if ctx.config.thresholds.app.checks.inheritance_depth_max > 0 {
        let ih = inheritance_depth::analyze(ctx.source, &ctx.module.language);
        for class in &ih.classes {
            if class.depth > ctx.config.thresholds.app.checks.inheritance_depth_max {
                facts.file_violations.push(ViolationReport {
                    metric: "inheritance_depth".to_string(),
                    actual: class.depth as f64,
                    limit: ctx.config.thresholds.app.checks.inheritance_depth_max as f64,
                    line: None,
                });
            }
        }
    }
}

/// Lazy test cluster detection for test-role files.
fn check_lazy_clusters(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts, role: Role) {
    if role == Role::Test && ctx.config.thresholds.test.checks.lazy_cluster_min > 0 {
        let clusters = cluster_detector::detect(
            &ctx.module.units,
            ctx.config.thresholds.test.checks.lazy_cluster_min,
        );
        for cluster in &clusters {
            facts.file_violations.push(ViolationReport {
                metric: "lazy_cluster".to_string(),
                actual: cluster.count as f64,
                limit: ctx.config.thresholds.test.checks.lazy_cluster_min as f64,
                line: None,
            });
        }
    }
}

/// Assemble the file report from the analysis outputs.
fn assemble_file_report(
    ctx: &FileCheckCtx<'_>,
    role: Role,
    diagnostics: Vec<Diagnostic>,
    facts: FileFacts,
) -> FileReport {
    FileReport {
        path: ctx.path.display().to_string(),
        role: role_name(role).to_string(),
        diagnostics: diagnostics.into_iter().map(Into::into).collect(),
        file_violations: facts.file_violations,
        units: facts.units,
        external_findings: vec![],
        mock_findings: facts
            .mock_analysis
            .references
            .into_iter()
            .map(|r| MockFindingReport {
                name: r.name,
                line: r.line,
                kind: format!("{:?}", r.kind),
            })
            .collect(),
    }
}

/// Display name for a detected role.
fn role_name(role: Role) -> &'static str {
    match role {
        Role::App => "app",
        Role::Library => "library",
        Role::Test => "test",
        Role::Script => "script",
        Role::Generated => "generated",
        Role::Vendor => "vendor",
        Role::UnitTest => "unit-test",
        Role::IntegrationTest => "integration-test",
    }
}
