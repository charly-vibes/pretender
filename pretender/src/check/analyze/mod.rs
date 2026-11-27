//! Per-file analysis: parse a source file and produce its report.
//!
//! Parses with the language-specific parser, detects the file's role, and
//! runs every file-level check (mock overuse, mutability, unwrap density,
//! primitive obsession, data model, inheritance depth, lazy clusters),
//! plus per-unit metric violations.
//!
//! # Responsibilities
//! Turn one source file into `(FileReport, Module)` or `None` when the
//! file is unreadable as UTF-8 or has no parser. The individual check
//! stages live in `super::analyze::checks`; unit-level metric violations
//! in `super::unit`; thresholds come from the role-merged effective
//! thresholds.
//!
//! # Rationale
//! Extracted verbatim from main.rs (pretender-5zo.2.2); the original
//! 203-line `analyze_path` was split into stage helpers sharing a
//! `FileCheckCtx` so every function lands within the structural limits
//! while check semantics and violation ordering stay identical. Split into
//! `mod.rs` plus `checks.rs` along the pipeline/check-stages seam
//! (pretender-5zo.15.12).
use super::scan::get_parser;
use super::unit::{build_unit_report, limit_violation};
use crate::report::{FileReport, MockFindingReport, UnitReport, ViolationReport};
use anyhow::{Context, Result};
use pretender::config::Config;
use pretender::mock_detector;
use pretender::model::{Diagnostic, Module};
use pretender::role_resolution::RoleDetector;
use pretender::roles::{EffectiveThresholds, Role};
use std::fs;
use std::path::Path;

mod checks;

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
    let Some(mut dm) = detect_module(path, detector)? else {
        return Ok(None);
    };
    let report = check_detected(path, &mut dm, config);
    Ok(Some((report, dm.module)))
}

/// Run every file-level check stage over a detected module and assemble its
/// report, in the original evaluation order.
fn check_detected(path: &Path, dm: &mut DetectedModule, config: &Config) -> FileReport {
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
    let diagnostics = std::mem::take(&mut dm.diagnostics);
    assemble_file_report(&ctx, dm.role, diagnostics, facts)
}

/// Run every file-level check stage in the original evaluation order,
/// starting with the file-level line-count limit.
fn run_file_checks(ctx: &FileCheckCtx<'_>, facts: &mut FileFacts, role: Role) {
    facts.file_violations.extend(limit_violation(
        "file_lines",
        ctx.module.lines_total,
        ctx.thresholds.file.file_lines_max,
    ));
    checks::check_mock_overuse(ctx, facts, role);
    checks::check_void_mutators(ctx, facts);
    checks::check_mut_ratio(ctx, facts);
    checks::check_unwrap_density(ctx, facts, role);
    checks::check_primitive_obsession(ctx, facts);
    checks::check_data_model(ctx, facts);
    checks::check_inheritance_depth(ctx, facts);
    checks::check_lazy_clusters(ctx, facts, role);
}

/// Read and parse the file, detect its role. `Ok(None)` when the source is
/// not valid UTF-8 or no parser handles the extension.
fn detect_module(path: &Path, detector: &RoleDetector) -> Result<Option<DetectedModule>> {
    let Some(source) = read_source(path)? else {
        return Ok(None);
    };
    let Ok(parser) = get_parser(path) else {
        return Ok(None);
    };
    let (module, diagnostics) = parser.parse(path, &source)?;
    let role = detector.detect(path, &source);
    let detected = DetectedModule {
        source,
        module,
        diagnostics,
        role,
    };
    Ok(Some(detected))
}

/// Read the source file; non-UTF-8 content is not parseable, not an error.
fn read_source(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(source) => Ok(Some(source)),
        Err(e) if e.kind() == std::io::ErrorKind::InvalidData => Ok(None),
        Err(e) => Err(e).context(format!("failed to read source file: {}", path.display())),
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
        mock_findings: mock_findings(facts.mock_analysis),
    }
}

/// The report view of the mock analysis references.
fn mock_findings(analysis: mock_detector::MockAnalysis) -> Vec<MockFindingReport> {
    analysis
        .references
        .into_iter()
        .map(|r| MockFindingReport {
            name: r.name,
            line: r.line,
            kind: format!("{:?}", r.kind),
        })
        .collect()
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
