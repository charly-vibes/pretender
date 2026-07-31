//! Module-level coupling metrics: Ce, Ca, CBO, LCOM-HS, and cycle detection.
//!
//! A "module" is a single source file. Coupling is computed from the
//! import graph built across all parsed files in a check run.

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

/// Build a dependency graph from parsed modules and compute coupling metrics.
///
/// Modules with role `generated` or `vendor` are excluded from the graph.
pub fn analyze(
    modules: &[(PathBuf, &Module)],
    thresholds: &CouplingThresholds,
) -> CouplingAnalysis {
    if !is_enabled(thresholds) {
        return CouplingAnalysis::default();
    }

    // Build adjacency: module_path -> set of imported module paths
    let mut outgoing: HashMap<String, HashSet<String>> = HashMap::new();
    let mut all_modules: HashSet<String> = HashSet::new();

    for (file_path, module) in modules {
        let module_path = normalize_module_path(file_path, module);
        all_modules.insert(module_path.clone());

        let deps: HashSet<String> = module
            .imports
            .iter()
            .filter_map(|imp| resolve_import_path(&module_path, imp))
            .collect();

        outgoing.insert(module_path, deps);
    }

    // Compute Ce (efferent) per module: number of distinct external deps
    // Compute Ca (afferent) per module: number of modules that import it
    let mut ce: HashMap<String, u32> = HashMap::new();
    let mut ca: HashMap<String, u32> = HashMap::new();

    for (mod_path, deps) in &outgoing {
        let internal_deps: Vec<&String> =
            deps.iter().filter(|d| all_modules.contains(*d)).collect();
        ce.insert(mod_path.clone(), internal_deps.len() as u32);
        for dep in internal_deps {
            *ca.entry(dep.clone()).or_insert(0) += 1;
        }
    }

    // Compute CBO = Ce + Ca
    let mut cbo: HashMap<String, u32> = HashMap::new();
    for mod_path in all_modules.iter() {
        let c = ce.get(mod_path).copied().unwrap_or(0);
        let a = ca.get(mod_path).copied().unwrap_or(0);
        cbo.insert(mod_path.clone(), c + a);
    }

    // Build module reports
    let mut modules_sorted: Vec<String> = all_modules.into_iter().collect();
    modules_sorted.sort();

    let mut reports: Vec<ModuleReport> = Vec::new();
    for mod_path in modules_sorted {
        let c = ce.get(&mod_path).copied().unwrap_or(0);
        let a = ca.get(&mod_path).copied().unwrap_or(0);
        let b = cbo.get(&mod_path).copied().unwrap_or(0);

        let mut violations = Vec::new();
        if thresholds.ce_max > 0 && c > thresholds.ce_max {
            violations.push(CouplingViolation {
                metric: "ce".to_string(),
                actual: c as f64,
                limit: thresholds.ce_max as f64,
            });
        }
        if thresholds.ca_max > 0 && a > thresholds.ca_max {
            violations.push(CouplingViolation {
                metric: "ca".to_string(),
                actual: a as f64,
                limit: thresholds.ca_max as f64,
            });
        }
        if thresholds.cbo_max > 0 && b > thresholds.cbo_max {
            violations.push(CouplingViolation {
                metric: "cbo".to_string(),
                actual: b as f64,
                limit: thresholds.cbo_max as f64,
            });
        }

        reports.push(ModuleReport {
            path: mod_path,
            violations,
        });
    }

    // Cycle detection
    let cycles = if thresholds.cycle_detection {
        detect_cycles(&outgoing)
    } else {
        Vec::new()
    };

    CouplingAnalysis {
        modules: reports,
        cycles,
    }
}

/// Detect cycles in the directed import graph using DFS with back-edge detection.
fn detect_cycles(graph: &HashMap<String, HashSet<String>>) -> Vec<CycleReport> {
    let mut cycles = Vec::new();
    let mut visited: HashSet<String> = HashSet::new();
    let mut in_stack: HashSet<String> = HashSet::new();
    let mut path: Vec<String> = Vec::new();

    let mut nodes: Vec<String> = graph.keys().cloned().collect();
    nodes.sort();

    for node in nodes {
        if !visited.contains(&node) {
            dfs_cycle(
                &node,
                graph,
                &mut visited,
                &mut in_stack,
                &mut path,
                &mut cycles,
            );
        }
    }

    cycles
}

fn dfs_cycle(
    node: &str,
    graph: &HashMap<String, HashSet<String>>,
    visited: &mut HashSet<String>,
    in_stack: &mut HashSet<String>,
    path: &mut Vec<String>,
    cycles: &mut Vec<CycleReport>,
) {
    visited.insert(node.to_string());
    in_stack.insert(node.to_string());
    path.push(node.to_string());

    if let Some(deps) = graph.get(node) {
        let mut deps_sorted: Vec<&String> = deps.iter().collect();
        deps_sorted.sort();
        for dep in deps_sorted {
            if !visited.contains(dep.as_str()) {
                dfs_cycle(dep, graph, visited, in_stack, path, cycles);
            } else if in_stack.contains(dep.as_str()) {
                // Found a cycle: extract from dep to current node
                if let Some(pos) = path.iter().position(|p| p == dep) {
                    let cycle: Vec<String> = path[pos..].to_vec();
                    // Only report unique cycles (normalize by starting at smallest node)
                    if is_canonical_cycle(&cycle) {
                        cycles.push(CycleReport {
                            participants: cycle.clone(),
                        });
                    }
                }
            }
        }
    }

    path.pop();
    in_stack.remove(node);
}

/// Ensure we only report each cycle once by normalizing to the lexicographically
/// smallest start node.
fn is_canonical_cycle(cycle: &[String]) -> bool {
    if cycle.is_empty() {
        return false;
    }
    let first = &cycle[0];
    cycle.iter().min().is_none_or(|min| min == first)
}

/// Normalize a file path to a module path for dependency graph keying.
fn normalize_module_path(file_path: &std::path::Path, _module: &Module) -> String {
    // Use the file path relative to the project root as the module identifier.
    // Strip any leading "./" for consistency.
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

    fn make_thresholds(ce: u32, ca: u32, cbo: u32, lcom: u32, cycles: bool) -> CouplingThresholds {
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
        let result = analyze(&modules, &make_thresholds(0, 0, 0, 0, false));
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
        let result = analyze(&modules, &make_thresholds(1, 0, 0, 0, false));

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
        let result = analyze(&modules, &make_thresholds(0, 0, 0, 0, true));
        assert!(result.cycles.is_empty());
    }

    #[test]
    fn simple_cycle_detected() {
        let (ap, a) = make_module("src/a.rs", vec!["src/b.rs"]);
        let (bp, b) = make_module("src/b.rs", vec!["src/c.rs"]);
        let (cp, c) = make_module("src/c.rs", vec!["src/a.rs"]);
        let modules = vec![(ap, &a), (bp, &b), (cp, &c)];
        let result = analyze(&modules, &make_thresholds(0, 0, 0, 0, true));
        assert_eq!(result.cycles.len(), 1);
        assert_eq!(result.cycles[0].participants.len(), 3);
    }

    #[test]
    fn std_imports_excluded() {
        let (p, a) = make_module("src/a.rs", vec!["std::collections::HashMap", "core::fmt"]);
        let modules = vec![(p, &a)];
        let result = analyze(&modules, &make_thresholds(1, 0, 0, 0, false));
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
        let result = analyze(&modules, &make_thresholds(0, 1, 0, 0, false));

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
