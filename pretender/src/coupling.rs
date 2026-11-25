//! Module-level coupling metrics: Ce, Ca, CBO, LCOM-HS, and cycle detection.
//! Purpose: surface structural coupling between modules so it can be gated on.
//! Responsibilities: build the dependency graph from parsed files, compute
//! Ce/Ca/CBO, emit per-module threshold violations, detect import cycles.
//! Rationale: a "module" is a single source file; coupling comes from the
//! import graph across all parsed files in a check run.

use crate::config::CouplingThresholds;

mod cycles;
use crate::model::{Import, Module};
use cycles::detect_cycles;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// A single finding for a module-level coupling violation.
#[derive(Debug, Clone, PartialEq)]
pub struct CouplingViolation {
    pub metric: String,
    pub actual: f64,
    pub limit: f64,
}

/// Result of coupling analysis for one module.
#[derive(Debug, Clone)]
pub struct ModuleReport {
    pub path: String,
    pub violations: Vec<CouplingViolation>,
}

/// Result of cycle detection.
#[derive(Debug, Clone)]
pub struct CycleReport {
    /// Ordered list of module paths forming the cycle.
    pub participants: Vec<String>,
}

/// Full coupling analysis result for a check run.
#[derive(Debug, Clone, Default)]
pub struct CouplingAnalysis {
    pub modules: Vec<ModuleReport>,
    pub cycles: Vec<CycleReport>,
}

/// Build the dependency graph and compute coupling metrics for all modules.
/// Modules with role `generated` or `vendor` are excluded upstream before
/// reaching this graph (contract enforced by the caller).
pub fn analyze(
    modules: &[(PathBuf, &Module)],
    thresholds: &CouplingThresholds,
) -> CouplingAnalysis {
    if !is_enabled(thresholds) {
        return CouplingAnalysis::default();
    }

    let graph = build_graph(modules);
    let counts = compute_coupling(&graph);
    finish_analysis(&graph, &counts, thresholds)
}

/// Sorted module reports plus optional cycle detection, assembled from a
/// built graph.
fn finish_analysis(
    graph: &DependencyGraph,
    counts: &CouplingCounts,
    thresholds: &CouplingThresholds,
) -> CouplingAnalysis {
    let mut modules_sorted: Vec<String> = graph.all_modules.iter().cloned().collect();
    modules_sorted.sort();
    let reports = counts.build_reports(&modules_sorted, thresholds);

    let cycles = if thresholds.cycle_detection {
        detect_cycles(&graph.outgoing)
    } else {
        Vec::new()
    };

    CouplingAnalysis {
        modules: reports,
        cycles,
    }
}

/// Dependency graph plus the set of known module paths.
struct DependencyGraph {
    outgoing: HashMap<String, HashSet<String>>,
    all_modules: HashSet<String>,
}

fn build_graph(modules: &[(PathBuf, &Module)]) -> DependencyGraph {
    let mut graph = DependencyGraph {
        outgoing: HashMap::new(),
        all_modules: HashSet::new(),
    };

    for (file_path, module) in modules {
        insert_module(&mut graph, file_path, module);
    }

    graph
}

/// Add one module and its resolved in-project imports to the graph.
fn insert_module(graph: &mut DependencyGraph, file_path: &Path, module: &Module) {
    let module_path = normalize_module_path(file_path, module);
    graph.all_modules.insert(module_path.clone());

    let deps: HashSet<String> = module
        .imports
        .iter()
        .filter_map(|imp| resolve_import_path(&module_path, imp))
        .collect();

    graph.outgoing.insert(module_path, deps);
}

/// Per-module coupling counts: efferent (Ce), afferent (Ca), and CBO.
struct CouplingCounts {
    ce: HashMap<String, u32>,
    ca: HashMap<String, u32>,
    cbo: HashMap<String, u32>,
}

impl CouplingCounts {
    /// One report per module in sorted order, with its threshold violations.
    fn build_reports(&self, mods: &[String], thresholds: &CouplingThresholds) -> Vec<ModuleReport> {
        let mut reports: Vec<ModuleReport> = Vec::new();
        for mod_path in mods {
            let ce = self.ce.get(mod_path).copied().unwrap_or(0);
            let ca = self.ca.get(mod_path).copied().unwrap_or(0);
            let cbo = self.cbo.get(mod_path).copied().unwrap_or(0);
            reports.push(ModuleReport {
                path: mod_path.clone(),
                violations: threshold_violations(ce, ca, cbo, thresholds),
            });
        }
        reports
    }
}

/// Compute Ce/Ca/CBO per module from the graph's in-project edges.
fn compute_coupling(graph: &DependencyGraph) -> CouplingCounts {
    let (ce, ca) = compute_ce_ca(graph);
    let cbo = compute_cbo(graph, &ce, &ca);
    CouplingCounts { ce, ca, cbo }
}

/// Efferent (out-going in-project) edges per module, and the mirrored
/// afferent count per imported module.
fn compute_ce_ca(graph: &DependencyGraph) -> (HashMap<String, u32>, HashMap<String, u32>) {
    let mut ce = HashMap::new();
    let mut ca = HashMap::new();

    for (mod_path, deps) in &graph.outgoing {
        let internal_deps: Vec<String> = deps.intersection(&graph.all_modules).cloned().collect();
        ce.insert(mod_path.clone(), internal_deps.len() as u32);
        for dep in &internal_deps {
            *ca.entry(dep.clone()).or_insert(0) += 1;
        }
    }

    (ce, ca)
}

/// CBO per module: efferent plus afferent count.
fn compute_cbo(
    graph: &DependencyGraph,
    ce: &HashMap<String, u32>,
    ca: &HashMap<String, u32>,
) -> HashMap<String, u32> {
    let mut cbo = HashMap::new();

    for mod_path in graph.all_modules.iter() {
        let c = ce.get(mod_path).copied().unwrap_or(0);
        let a = ca.get(mod_path).copied().unwrap_or(0);
        cbo.insert(mod_path.clone(), c + a);
    }

    cbo
}

/// Violations of the enabled ce/ca/cbo thresholds, in that order.
fn threshold_violations(
    ce: u32,
    ca: u32,
    cbo: u32,
    thresholds: &CouplingThresholds,
) -> Vec<CouplingViolation> {
    [
        ("ce", ce, thresholds.ce_max),
        ("ca", ca, thresholds.ca_max),
        ("cbo", cbo, thresholds.cbo_max),
    ]
    .iter()
    .filter(|&&(_, actual, limit)| limit > 0 && actual > limit)
    .map(|&(metric, actual, limit)| CouplingViolation {
        metric: metric.to_string(),
        actual: actual as f64,
        limit: limit as f64,
    })
    .collect()
}

/// Normalize a file path to a module path for dependency graph keying.
fn normalize_module_path(file_path: &std::path::Path, _module: &Module) -> String {
    let s = file_path.display().to_string();
    s.strip_prefix("./").unwrap_or(&s).to_string()
}

/// Resolve an import to a module path. Returns None for external/library
/// imports that can't be resolved to a project module.
fn resolve_import_path(_current_module: &str, imp: &Import) -> Option<String> {
    // For now, return the module string as-is. The coupling analysis only
    // counts connections between modules that are both in the parsed set.
    let module = imp.module.trim_matches('"').trim();
    let skipped = module.is_empty() || module.starts_with("std::") || module.starts_with("core::");
    (!skipped).then(|| module.to_string())
}

/// Check if any coupling threshold is enabled.
fn is_enabled(thresholds: &CouplingThresholds) -> bool {
    [
        thresholds.ce_max,
        thresholds.ca_max,
        thresholds.cbo_max,
        thresholds.lcom_hs_max,
    ]
    .into_iter()
    .any(|limit| limit > 0)
        || thresholds.cycle_detection
}

#[cfg(test)]
mod tests;
