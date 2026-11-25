//! DFS cycle detection over the module import graph.
//!
//! Purpose: find import cycles and report each exactly once.
//! Responsibilities: walk the graph depth-first in sorted order and extract
//! canonical back-edge cycles. Sibling of the graph/counts logic in the
//! parent `coupling` module; split so each file stays under the file cap.
//! The traversal state is owned (not borrowed) so the driver needs a
//! single mutable binding.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::CycleReport;

/// Detect cycles in the directed import graph using DFS with back-edge detection.
pub(super) fn detect_cycles(graph: &HashMap<String, HashSet<String>>) -> Vec<CycleReport> {
    let mut state = DfsState {
        graph,
        visited: HashSet::new(),
        in_stack: HashSet::new(),
        path: Vec::new(),
        cycles: Vec::new(),
    };

    let roots = sorted_nodes(graph);
    for node in roots {
        if !state.visited.contains(&node) {
            state.walk(&node);
        }
    }

    state.cycles
}

/// Graph nodes in sorted order, pinning DFS visit order. A `BTreeSet`
/// yields the sorted sequence without a mutable sort binding.
fn sorted_nodes(graph: &HashMap<String, HashSet<String>>) -> Vec<String> {
    graph
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Mutable traversal state shared by the recursive `walk`.
struct DfsState<'a> {
    graph: &'a HashMap<String, HashSet<String>>,
    visited: HashSet<String>,
    in_stack: HashSet<String>,
    path: Vec<String>,
    cycles: Vec<CycleReport>,
}

impl DfsState<'_> {
    fn walk(&mut self, node: &str) {
        self.visited.insert(node.to_string());
        self.in_stack.insert(node.to_string());
        self.path.push(node.to_string());

        self.walk_deps(node);

        self.path.pop();
        self.in_stack.remove(node);
    }

    /// Visit the node's dependencies in sorted order.
    fn walk_deps(&mut self, node: &str) {
        let deps = self.graph.get(node);
        if let Some(deps) = deps {
            for dep in deps.iter().collect::<BTreeSet<_>>() {
                self.visit(dep);
            }
        }
    }

    /// Walk one edge: recurse into unvisited deps, report back-edges as cycles.
    fn visit(&mut self, dep: &str) {
        if !self.visited.contains(dep) {
            self.walk(dep);
        } else if self.in_stack.contains(dep) {
            self.report_back_edge(dep);
        }
    }

    /// `dep` closes a path back into the DFS stack: extract that path as a cycle.
    fn report_back_edge(&mut self, dep: &str) {
        if let Some(pos) = self.path.iter().position(|p| p == dep) {
            let cycle: Vec<String> = self.path[pos..].to_vec();
            if is_canonical_cycle(&cycle) {
                self.cycles.push(CycleReport {
                    participants: cycle,
                });
            }
        }
    }
}

/// Report each cycle once: canonical iff it starts at its smallest member.
/// (Empty cycles can't occur — always extracted from a non-empty path.)
fn is_canonical_cycle(cycle: &[String]) -> bool {
    !cycle.is_empty() && cycle.first() == cycle.iter().min()
}
