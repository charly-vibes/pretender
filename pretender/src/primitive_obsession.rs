//! Primitive obsession and boolean cluster detection.
//!
//! Purpose: detect excessive bare boolean params and primitive types used
//! in place of domain types. Responsibilities: per-unit violation analysis
//! plus the shared bare-primitive/domain-name predicates. Rationale:
//! config-driven heuristics consumed by engine reporting.

use crate::model::{CodeUnit, Parameter, UnitKind};

/// Result of primitive-obsession analysis for one code unit.
#[derive(Debug, Clone, Default)]
pub struct PrimitiveObsessionAnalysis {
    pub bool_param_count: u32,
    pub violations: Vec<PrimitiveViolation>,
}

/// A single violation finding.
#[derive(Debug, Clone)]
pub struct PrimitiveViolation {
    pub kind: PrimitiveViolationKind,
    #[allow(dead_code)] // informative metadata for future output formats
    pub unit_name: String,
    #[allow(dead_code)] // informative metadata for future output formats
    pub param_name: String,
    #[allow(dead_code)] // informative metadata for future output formats
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrimitiveViolationKind {
    BoolCluster,
    PrimitiveDomainParam,
}

/// Per-run heuristic configuration shared by the analysis helpers.
struct UnitChecks<'a> {
    bool_cluster_max: u32,
    primitive_param_check: bool,
    extra_domain_patterns: &'a [String],
}

/// Analyze code units for primitive-obsession violations.
///
/// * `bool_cluster_max` — max number of bare bool params before flagging (0 = disabled)
/// * `primitive_param_check` — check for String/int params with domain-like names
/// * `extra_domain_patterns` — additional domain name patterns from config
pub fn analyze(
    units: &[CodeUnit],
    bool_cluster_max: u32,
    primitive_param_check: bool,
    extra_domain_patterns: &[String],
) -> Vec<PrimitiveObsessionAnalysis> {
    let cfg = UnitChecks {
        bool_cluster_max,
        primitive_param_check,
        extra_domain_patterns,
    };
    units.iter().map(|u| analyze_unit(u, &cfg)).collect()
}

/// Per-unit driver: skips ctor/initializer defs (empty analysis still
/// emitted). Verbatim extraction; early return replaces the negated `if`.
fn analyze_unit(unit: &CodeUnit, cfg: &UnitChecks<'_>) -> PrimitiveObsessionAnalysis {
    let mut analysis = PrimitiveObsessionAnalysis::default();

    // Skip struct/class/record definitions — only check function/method params
    if matches!(unit.kind, UnitKind::Ctor | UnitKind::Initializer) {
        return analysis;
    }

    count_bool_and_domain_params(unit, cfg, &mut analysis);
    flag_bool_cluster(unit, cfg.bool_cluster_max, &mut analysis);

    analysis
}

/// Counting pass: bool params counted, PrimitiveDomainParam pushed in
/// parameter order before any BoolCluster (order pinned by tests).
fn count_bool_and_domain_params(
    unit: &CodeUnit,
    cfg: &UnitChecks<'_>,
    analysis: &mut PrimitiveObsessionAnalysis,
) {
    for param in &unit.parameters {
        if param_counts_as_bool(param) {
            analysis.bool_param_count += 1;
        }

        // Check for primitive domain params
        if cfg.primitive_param_check {
            flag_primitive_domain_param(unit, param, cfg, analysis);
        }
    }
}

/// Param-level counting classification; `None` annotations never count
/// (preserved from the original `map(..).unwrap_or(false)` closure).
fn param_counts_as_bool(param: &Parameter) -> bool {
    param
        .type_name
        .as_deref()
        .map(is_bool_counting_type)
        .unwrap_or(false)
}

/// Cluster-eligible boolean spellings; verbatim from the original
/// cluster-emission closure.
fn is_bool_cluster_type(type_name: &str) -> bool {
    let t = type_name.trim();
    t == "bool" || t == "boolean" || t == "Boolean" || t == "bool?" || t == "Bool"
}

/// Counted boolean spellings: the cluster set plus `boolish`, which counts
/// toward the threshold but is never flagged (asymmetry pinned by tests).
fn is_bool_counting_type(type_name: &str) -> bool {
    is_bool_cluster_type(type_name) || type_name.trim() == "boolish"
}

/// Domain-param check for one parameter; `let-else` unwinds the original
/// nesting depth 6 to 2 with identical semantics.
fn flag_primitive_domain_param(
    unit: &CodeUnit,
    param: &Parameter,
    cfg: &UnitChecks<'_>,
    analysis: &mut PrimitiveObsessionAnalysis,
) {
    let Some(type_name) = &param.type_name else {
        return;
    };
    if is_bare_primitive_type(type_name)
        && is_domain_param_name(&param.name, cfg.extra_domain_patterns)
    {
        analysis.violations.push(PrimitiveViolation {
            kind: PrimitiveViolationKind::PrimitiveDomainParam,
            unit_name: unit.name.clone(),
            param_name: param.name.clone(),
            line: param.span.start_line,
        });
    }
}

/// One violation finding for a parameter of `unit`.
fn primitive_violation(
    kind: PrimitiveViolationKind,
    unit: &CodeUnit,
    param: &Parameter,
) -> PrimitiveViolation {
    PrimitiveViolation {
        kind,
        unit_name: unit.name.clone(),
        param_name: param.name.clone(),
        line: param.span.start_line,
    }
}

/// Bool-cluster emission: at threshold (0 disables) push one BoolCluster
/// per standard-bool param; guard is the original conjunction inverted.
fn flag_bool_cluster(
    unit: &CodeUnit,
    bool_cluster_max: u32,
    analysis: &mut PrimitiveObsessionAnalysis,
) {
    if bool_cluster_max == 0 || analysis.bool_param_count < bool_cluster_max {
        return;
    }

    for param in &unit.parameters {
        if param.type_name.as_deref().is_some_and(is_bool_cluster_type) {
            analysis.violations.push(primitive_violation(
                PrimitiveViolationKind::BoolCluster,
                unit,
                param,
            ));
        }
    }
}

/// Check if a declared type name is a bare primitive (no domain type).
///
/// Shared notion used by the parameter-level check here and the
/// field-level check in `data_model`.
pub(crate) fn is_bare_primitive_type(type_name: &str) -> bool {
    matches!(
        type_name.trim(),
        "String"
            | "str"
            | "int"
            | "i32"
            | "i64"
            | "u32"
            | "u64"
            | "Int"
            | "Integer"
            | "number"
            | "string"
    )
}

/// Domain name patterns that suggest a primitive should be a domain type.
const DOMAIN_PATTERNS: [&str; 12] = [
    "email",
    "url",
    "uri",
    "date",
    "timestamp",
    "phone",
    "address",
    "zip",
    "ssn",
    "uuid",
    "token",
    "secret",
];

/// Check if a parameter name suggests a domain concept that should be a type.
///
/// Excludes common false positives: `id`, `name`, `path` — these are
/// legitimate primitive uses in most contexts.
pub(crate) fn is_domain_param_name(name: &str, extra_patterns: &[String]) -> bool {
    let lower = name.to_ascii_lowercase();
    DOMAIN_PATTERNS
        .iter()
        .copied()
        .chain(extra_patterns.iter().map(|s| s.as_str()))
        .any(|pattern| lower.contains(pattern))
}
