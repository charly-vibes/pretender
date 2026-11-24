//! Module-level coupling metrics: Ce, Ca, CBO, LCOM-HS, and cycle detection.
//! Purpose: surface structural coupling between modules so it can be gated on.
//! Responsibilities: build the dependency graph from parsed files, compute
//! Ce/Ca/CBO, emit per-module threshold violations, detect import cycles.
//! Rationale: a "module" is a single source file; coupling comes from the
//! import graph across all parsed files in a check run.

use crate::config::CouplingThresholds;
use crate::model::{Import, Module};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

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
        let module_path = normalize_module_path(file_path, module);
        graph.all_modules.insert(module_path.clone());

        let deps: HashSet<String> = module
            .imports
            .iter()
            .filter_map(|imp| resolve_import_path(&module_path, imp))
            .collect();

        graph.outgoing.insert(module_path, deps);
    }

    graph
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
    let mut ce = HashMap::new();
    let mut ca = HashMap::new();
    let mut cbo = HashMap::new();

    for (mod_path, deps) in &graph.outgoing {
        let internal_deps: Vec<String> = deps.intersection(&graph.all_modules).cloned().collect();
        ce.insert(mod_path.clone(), internal_deps.len() as u32);
        for dep in &internal_deps {
            *ca.entry(dep.clone()).or_insert(0) += 1;
        }
    }

    for mod_path in graph.all_modules.iter() {
        let c = ce.get(mod_path).copied().unwrap_or(0);
        let a = ca.get(mod_path).copied().unwrap_or(0);
        cbo.insert(mod_path.clone(), c + a);
    }

    CouplingCounts { ce, ca, cbo }
}

/// Violations of the enabled ce/ca/cbo thresholds, in that order.
fn threshold_violations(
    ce: u32,
    ca: u32,
    cbo: u32,
    thresholds: &CouplingThresholds,
) -> Vec<CouplingViolation> {
    let mut violations = Vec::new();
    for (metric, actual, limit) in [
        ("ce", ce, thresholds.ce_max),
        ("ca", ca, thresholds.ca_max),
        ("cbo", cbo, thresholds.cbo_max),
    ] {
        if limit > 0 && actual > limit {
            violations.push(CouplingViolation {
                metric: metric.to_string(),
                actual: actual as f64,
                limit: limit as f64,
            });
        }
    }
    violations
}

/// Detect cycles in the directed import graph using DFS with back-edge detection.
fn detect_cycles(graph: &HashMap<String, HashSet<String>>) -> Vec<CycleReport> {
    let mut cycles = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut in_stack: HashSet<String> = HashSet::new();
    let mut path: Vec<String> = Vec::new();
    let mut state = DfsState {
        graph,
        visited: &mut visited,
        in_stack: &mut in_stack,
        path: &mut path,
        cycles: &mut cycles,
    };

    let mut nodes: Vec<String> = graph.keys().cloned().collect();
    nodes.sort();

    for node in nodes {
        if !state.visited.contains(&node) {
            dfs_cycle(&mut state, &node);
        }
    }

    cycles
}

/// Mutable traversal state shared by the recursive `dfs_cycle` walk.
struct DfsState<'a> {
    graph: &'a HashMap<String, HashSet<String>>,
    visited: &'a mut HashSet<String>,
    in_stack: &'a mut HashSet<String>,
    path: &'a mut Vec<String>,
    cycles: &'a mut Vec<CycleReport>,
}

fn dfs_cycle(state: &mut DfsState<'_>, node: &str) {
    state.visited.insert(node.to_string());
    state.in_stack.insert(node.to_string());
    state.path.push(node.to_string());

    if let Some(deps) = state.graph.get(node) {
        let mut deps_sorted: Vec<&String> = deps.iter().collect();
        deps_sorted.sort();
        for dep in deps_sorted {
            visit_dependency(state, dep);
        }
    }

    state.path.pop();
    state.in_stack.remove(node);
}

/// Walk one edge: recurse into unvisited deps, report back-edges as cycles.
fn visit_dependency(state: &mut DfsState<'_>, dep: &str) {
    if !state.visited.contains(dep) {
        dfs_cycle(state, dep);
    } else if state.in_stack.contains(dep) {
        report_back_edge(state, dep);
    }
}

/// `dep` closes a path back into the DFS stack: extract that path as a cycle.
fn report_back_edge(state: &mut DfsState<'_>, dep: &str) {
    if let Some(pos) = state.path.iter().position(|p| p == dep) {
        let cycle: Vec<String> = state.path[pos..].to_vec();
        if is_canonical_cycle(&cycle) {
            state.cycles.push(CycleReport {
                participants: cycle,
            });
        }
    }
}

/// Report each cycle once: canonical iff it starts at its smallest member.
/// (Empty cycles can't occur — always extracted from a non-empty path.)
fn is_canonical_cycle(cycle: &[String]) -> bool {
    !cycle.is_empty() && cycle.first() == cycle.iter().min()
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
    if module.is_empty() || module.starts_with("std::") || module.starts_with("core::") {
        return None; // Skip standard library imports
    }
    Some(module.to_string())
}

/// Check if any coupling threshold is enabled.
fn is_enabled(thresholds: &CouplingThresholds) -> bool {
    thresholds.ce_max > 0
        || thresholds.ca_max > 0
        || thresholds.cbo_max > 0
        || thresholds.lcom_hs_max > 0
        || thresholds.cycle_detection
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Import, Language, Span};

    fn make_module(path: &str, imports: Vec<&str>) -> (PathBuf, Module) {
        let path = PathBuf::from(path);
        let module = Module {
            path: path.clone(),
            language: Language::Rust,
            span: Span {
                start_line: 1,
                end_line: 10,
            },
            lines_total: 10,
            lines_code: 8,
            lines_comment: 2,
            units: vec![],
            type_definitions: Vec::new(),
            imports: imports
                .into_iter()
                .map(|i| Import {
                    module: i.to_string(),
                    name: None,
                    alias: None,
                    span: Span {
                        start_line: 1,
                        end_line: 1,
                    },
                })
                .collect(),
        };
        (path, module)
    }

    fn make_thresholds(counts: (u32, u32, u32, u32), cycles: bool) -> CouplingThresholds {
        let (ce, ca, cbo, lcom) = counts;
        CouplingThresholds {
            ce_max: ce,
            ca_max: ca,
            cbo_max: cbo,
            lcom_hs_max: lcom,
            cycle_detection: cycles,
        }
    }

    #[test]
    fn no_thresholds_no_analysis() {
        let (p, m) = make_module("src/main.rs", vec!["std::collections::HashMap"]);
        let modules = vec![(p, &m)];
        let result = analyze(&modules, &make_thresholds((0, 0, 0, 0), false));
        assert!(result.modules.is_empty());
        assert!(result.cycles.is_empty());
    }

    #[test]
    fn ce_threshold_violation() {
        // Module A imports B and C (Ce=2), ce_max=1
        let (ap, a) = make_module("src/a.rs", vec!["src/b.rs", "src/c.rs"]);
        let (bp, b) = make_module("src/b.rs", vec![]);
        let (cp, c) = make_module("src/c.rs", vec![]);
        let modules = vec![(ap, &a), (bp, &b), (cp, &c)];
        let result = analyze(&modules, &make_thresholds((1, 0, 0, 0), false));

        let a_report = result
            .modules
            .iter()
            .find(|r| r.path.contains("a.rs"))
            .unwrap();
        assert_eq!(a_report.violations.len(), 1);
        assert_eq!(a_report.violations[0].metric, "ce");
        assert_eq!(a_report.violations[0].actual, 2.0);
    }

    #[test]
    fn no_cycle_in_acyclic_graph() {
        let (ap, a) = make_module("src/a.rs", vec!["src/b.rs"]);
        let (bp, b) = make_module("src/b.rs", vec!["src/c.rs"]);
        let (cp, c) = make_module("src/c.rs", vec![]);
        let modules = vec![(ap, &a), (bp, &b), (cp, &c)];
        let result = analyze(&modules, &make_thresholds((0, 0, 0, 0), true));
        assert!(result.cycles.is_empty());
    }

    #[test]
    fn simple_cycle_detected() {
        let (ap, a) = make_module("src/a.rs", vec!["src/b.rs"]);
        let (bp, b) = make_module("src/b.rs", vec!["src/c.rs"]);
        let (cp, c) = make_module("src/c.rs", vec!["src/a.rs"]);
        let modules = vec![(ap, &a), (bp, &b), (cp, &c)];
        let result = analyze(&modules, &make_thresholds((0, 0, 0, 0), true));
        assert_eq!(result.cycles.len(), 1);
        assert_eq!(result.cycles[0].participants.len(), 3);
    }

    #[test]
    fn std_imports_excluded() {
        let (p, a) = make_module("src/a.rs", vec!["std::collections::HashMap", "core::fmt"]);
        let modules = vec![(p, &a)];
        let result = analyze(&modules, &make_thresholds((1, 0, 0, 0), false));
        assert!(
            result.modules.is_empty() || result.modules.iter().all(|r| r.violations.is_empty())
        );
    }

    #[test]
    fn ca_threshold_violation() {
        // Module B is imported by A and C (Ca=2), ca_max=1
        let (ap, a) = make_module("src/a.rs", vec!["src/b.rs"]);
        let (bp, b) = make_module("src/b.rs", vec![]);
        let (cp, c) = make_module("src/c.rs", vec!["src/b.rs"]);
        let modules = vec![(ap, &a), (bp, &b), (cp, &c)];
        let result = analyze(&modules, &make_thresholds((0, 1, 0, 0), false));

        let b_report = result
            .modules
            .iter()
            .find(|r| r.path.contains("b.rs"))
            .unwrap();
        assert_eq!(b_report.violations.len(), 1);
        assert_eq!(b_report.violations[0].metric, "ca");
        assert_eq!(b_report.violations[0].actual, 2.0);
    }
}
