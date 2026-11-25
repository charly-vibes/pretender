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

/// Per-block tallies gathered by `collect_stats`. `branches` is a
/// `BTreeMap` so fingerprint output ordering needs no separate sort.
#[derive(Default)]
struct BlockStats {
    call_count: u32,
    callee_names: Vec<String>,
    branch_counts: std::collections::BTreeMap<String, u32>,
}

impl BlockStats {
    /// Fold one node into the tallies.
    fn absorb(&mut self, child: &Node) {
        match child {
            Node::Call(call) => {
                self.call_count += 1;
                self.callee_names.push(call.callee.clone());
            }
            Node::Branch(branch) => {
                let name = format!("{:?}", branch.kind);
                *self.branch_counts.entry(name).or_insert(0) += 1;
            }
            _ => {}
        }
    }

    /// Merge nested-block tallies into this one.
    fn merge(&mut self, nested: BlockStats) {
        self.call_count += nested.call_count;
        self.callee_names.extend(nested.callee_names);
        for (name, count) in nested.branch_counts {
            *self.branch_counts.entry(name).or_insert(0) += count;
        }
    }
}

/// Build the structural fingerprint of a single unit.
fn fingerprint_unit(unit: &CodeUnit) -> Fingerprint {
    finish_fingerprint(collect_stats(&unit.body), unit.assertions)
}

/// Sort callee names and branch kinds so structurally identical units
/// produce byte-identical fingerprints.
fn finish_fingerprint(mut stats: BlockStats, assertion_count: u32) -> Fingerprint {
    stats.callee_names.sort();

    Fingerprint {
        call_count: stats.call_count,
        callee_names: stats.callee_names,
        assertion_count,
        branch_counts: stats.branch_counts.into_iter().collect(),
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
    fingerprints
        .iter()
        .fold(std::collections::HashMap::new(), |mut groups, (idx, fp)| {
            let key = format!(
                "{}-{}-{:?}-{:?}",
                fp.call_count, fp.assertion_count, fp.callee_names, fp.branch_counts
            );
            groups.entry(key).or_default().push(*idx);
            groups
        })
}

/// Convert each group with at least `min_size` members into a cluster.
fn clusters_from_groups(
    groups: std::collections::HashMap<String, Vec<usize>>,
    units: &[CodeUnit],
    min_size: u32,
) -> Vec<LazyCluster> {
    groups
        .into_values()
        .filter(|indices| indices.len() >= min_size as usize)
        .map(|indices| cluster_from_indices(&indices, units))
        .collect()
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

/// Recursively tally calls and branch kinds over a block tree.
fn collect_stats(block: &crate::model::Block) -> BlockStats {
    block
        .children
        .iter()
        .fold(BlockStats::default(), |mut acc, child| {
            match child {
                Node::NestedBlock(nested) => acc.merge(collect_stats(nested)),
                node => acc.absorb(node),
            }
            acc
        })
}

#[cfg(test)]
mod tests;
