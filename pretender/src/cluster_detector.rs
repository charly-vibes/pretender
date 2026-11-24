//! Lazy test cluster detection.
//!
//! Groups test code units with identical assertion structure and SUT call
//! differing only in literal values. Single-file analysis only.
//!
//! # Responsibilities
//! Fingerprint each unit's structure (SUT calls, assertion count, branch
//! kinds), group units with identical fingerprints, and emit clusters of at
//! least `min_size` members, sorted by member count descending (stable sort).
//!
//! # Rationale
//! `detect` is split into stage helpers so every function stays within the
//! gate's function-size limit; bodies were moved verbatim to preserve
//! detection semantics and output ordering exactly.

use crate::model::{CodeUnit, Node};

/// A cluster of similar test code units.
#[derive(Debug, Clone)]
pub struct LazyCluster {
    #[allow(dead_code)] // informative metadata for future output formats
    pub sut_name: String,
    pub count: u32,
    #[allow(dead_code)] // informative metadata for future output formats
    pub lines: Vec<u32>,
}

/// Analyze test code units for lazy test clusters.
///
/// A cluster is a group of ≥ `min_size` code units with the same:
/// 1. Called function / SUT (from call sites)
/// 2. Assertion count and types
/// 3. Branching structure (counts per branch kind)
///
/// Only literal values differ between members of a cluster.
pub fn detect(units: &[CodeUnit], min_size: u32) -> Vec<LazyCluster> {
    if min_size < 2 || units.len() < min_size as usize {
        return Vec::new();
    }

    let fingerprints = build_fingerprints(units);
    let groups = group_by_fingerprint(&fingerprints);
    let mut clusters = clusters_from_groups(groups, units, min_size);

    clusters.sort_by_key(|c| std::cmp::Reverse(c.count));
    clusters
}

/// Structural fingerprint of one code unit: SUT call count and names,
/// assertion count, and branch-kind counts. Units with identical
/// fingerprints differ only in literal values.
struct Fingerprint {
    call_count: u32,
    callee_names: Vec<String>,
    assertion_count: u32,
    branch_counts: Vec<(String, u32)>,
}

/// Build the structural fingerprint of a single unit.
fn fingerprint_unit(unit: &CodeUnit) -> Fingerprint {
    let mut call_count = 0u32;
    let mut callee_names = Vec::new();
    let assertion_count = unit.assertions;
    let mut branch_counts: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();

    collect_stats(
        &unit.body,
        &mut call_count,
        &mut callee_names,
        &mut branch_counts,
    );

    let mut branch_counts_vec: Vec<(String, u32)> = branch_counts.into_iter().collect();
    branch_counts_vec.sort_by(|a, b| a.0.cmp(&b.0));

    callee_names.sort();

    Fingerprint {
        call_count,
        callee_names,
        assertion_count,
        branch_counts: branch_counts_vec,
    }
}

/// Fingerprint every unit in order, pairing each fingerprint with its index.
fn build_fingerprints(units: &[CodeUnit]) -> Vec<(usize, Fingerprint)> {
    units
        .iter()
        .enumerate()
        .map(|(idx, unit)| (idx, fingerprint_unit(unit)))
        .collect()
}

/// Group unit indices by fingerprint key; indices keep unit order.
fn group_by_fingerprint(
    fingerprints: &[(usize, Fingerprint)],
) -> std::collections::HashMap<String, Vec<usize>> {
    let mut groups: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();

    for (idx, fp) in fingerprints {
        let key = format!(
            "{}-{}-{:?}-{:?}",
            fp.call_count, fp.assertion_count, fp.callee_names, fp.branch_counts
        );
        groups.entry(key).or_default().push(*idx);
    }

    groups
}

/// Convert each group with at least `min_size` members into a cluster.
fn clusters_from_groups(
    groups: std::collections::HashMap<String, Vec<usize>>,
    units: &[CodeUnit],
    min_size: u32,
) -> Vec<LazyCluster> {
    let mut clusters = Vec::new();
    for (_key, indices) in groups {
        if indices.len() >= min_size as usize {
            clusters.push(cluster_from_indices(&indices, units));
        }
    }
    clusters
}

/// Build one cluster from a group of unit indices (first member names the SUT).
fn cluster_from_indices(indices: &[usize], units: &[CodeUnit]) -> LazyCluster {
    let first_idx = indices[0];
    let unit = &units[first_idx];
    let sut_name = if unit.assertions > 0 {
        format!("{} (test)", unit.name)
    } else {
        unit.name.clone()
    };

    let lines: Vec<u32> = indices.iter().map(|i| units[*i].span.start_line).collect();

    LazyCluster {
        sut_name,
        count: indices.len() as u32,
        lines,
    }
}

fn collect_stats(
    block: &crate::model::Block,
    call_count: &mut u32,
    callee_names: &mut Vec<String>,
    branch_counts: &mut std::collections::HashMap<String, u32>,
) {
    for child in &block.children {
        match child {
            Node::Call(call) => {
                *call_count += 1;
                callee_names.push(call.callee.clone());
            }
            Node::Branch(branch) => {
                let name = format!("{:?}", branch.kind);
                *branch_counts.entry(name).or_insert(0) += 1;
            }
            Node::NestedBlock(nested) => {
                collect_stats(nested, call_count, callee_names, branch_counts);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Block, Branch, BranchKind, CallSite, Node, Span};

    fn make_call(callee: &str) -> Node {
        Node::Call(CallSite {
            callee: callee.to_string(),
            span: Span {
                start_line: 1,
                end_line: 1,
            },
            smell_weight: 1.0,
        })
    }

    fn make_branch(kind: BranchKind) -> Node {
        Node::Branch(Branch {
            kind,
            span: Span {
                start_line: 1,
                end_line: 1,
            },
            nesting_at: 0,
            sequence_id: None,
            cyclomatic_weight: 1,
            cognitive_weight: 1,
        })
    }

    fn make_unit(name: &str, body_children: Vec<Node>, assertions: u32, start: u32) -> CodeUnit {
        CodeUnit {
            name: name.to_string(),
            kind: crate::model::UnitKind::Function,
            span: Span {
                start_line: start,
                end_line: start + 2,
            },
            parameters: vec![],
            body: Block {
                span: Span {
                    start_line: start,
                    end_line: start + 2,
                },
                nesting: 0,
                children: body_children,
            },
            is_exported: false,
            assertions,
            parent_class: None,
            has_test_attribute: false,
        }
    }

    #[test]
    fn detects_cluster_of_three() {
        let units = vec![
            make_unit(
                "test_add_1",
                vec![make_call("add"), make_branch(BranchKind::If)],
                2,
                1,
            ),
            make_unit(
                "test_add_2",
                vec![make_call("add"), make_branch(BranchKind::If)],
                2,
                5,
            ),
            make_unit(
                "test_add_3",
                vec![make_call("add"), make_branch(BranchKind::If)],
                2,
                9,
            ),
        ];
        let clusters = detect(&units, 3);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].count, 3);
    }

    #[test]
    fn different_structure_not_clustered() {
        let units = vec![
            make_unit(
                "test_a",
                vec![make_call("add"), make_branch(BranchKind::If)],
                2,
                1,
            ),
            make_unit(
                "test_b",
                vec![
                    make_call("add"),
                    make_branch(BranchKind::If),
                    make_branch(BranchKind::Loop),
                ],
                2,
                5,
            ),
            make_unit(
                "test_c",
                vec![make_call("add"), make_branch(BranchKind::If)],
                2,
                9,
            ),
        ];
        let clusters = detect(&units, 3);
        // Different branch structure -> no cluster of 3
        assert!(clusters.is_empty() || clusters.iter().all(|c| c.count < 3));
    }

    #[test]
    fn below_threshold_not_clustered() {
        let units = vec![
            make_unit("test_a", vec![make_call("add")], 1, 1),
            make_unit("test_b", vec![make_call("add")], 1, 3),
        ];
        let clusters = detect(&units, 3);
        assert!(clusters.is_empty());
    }

    #[test]
    fn empty_units_no_clusters() {
        let clusters = detect(&[], 3);
        assert!(clusters.is_empty());
    }

    #[test]
    fn min_size_two_detects_pair() {
        let units = vec![
            make_unit("test_a", vec![make_call("add")], 1, 1),
            make_unit("test_b", vec![make_call("add")], 1, 3),
        ];
        let clusters = detect(&units, 2);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].count, 2);
    }

    #[test]
    fn cluster_ordering_pinned() {
        // Pins detector output ordering: clusters are sorted by member count
        // descending, and member lines follow unit order within a cluster.
        let units = vec![
            make_unit("test_add_1", vec![make_call("add")], 1, 1),
            make_unit("test_add_2", vec![make_call("add")], 1, 5),
            make_unit("test_add_3", vec![make_call("add")], 1, 9),
            make_unit("test_sub_1", vec![make_call("sub")], 1, 13),
            make_unit("test_sub_2", vec![make_call("sub")], 1, 17),
        ];
        let clusters = detect(&units, 2);
        assert_eq!(clusters.len(), 2);
        assert_eq!(clusters[0].count, 3);
        assert_eq!(clusters[0].lines, vec![1, 5, 9]);
        assert_eq!(clusters[1].count, 2);
        assert_eq!(clusters[1].lines, vec![13, 17]);
    }
}
