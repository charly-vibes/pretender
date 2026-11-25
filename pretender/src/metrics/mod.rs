use crate::model::{Block, BranchKind, CodeUnit, Metric, Node};
use std::collections::HashSet;

pub struct CyclomaticComplexity;

impl Metric for CyclomaticComplexity {
    fn calculate(&self, unit: &CodeUnit) -> u32 {
        cyclomatic(unit)
    }
}

pub fn cyclomatic(unit: &CodeUnit) -> u32 {
    1 + count_branches(&unit.body)
}

pub fn cognitive(unit: &CodeUnit) -> u32 {
    cognitive_block(&unit.body, &mut HashSet::new())
}

pub fn function_lines(unit: &CodeUnit) -> u32 {
    unit.span.lines()
}

pub fn params(unit: &CodeUnit) -> u32 {
    unit.parameters.len() as u32
}

pub fn nesting_max(unit: &CodeUnit) -> u32 {
    nesting_max_block(&unit.body)
}

pub fn abc(unit: &CodeUnit) -> f64 {
    let totals = abc_block(&unit.body);
    (totals.assignments * totals.assignments
        + totals.branches * totals.branches
        + totals.calls * totals.calls)
        .sqrt()
}

fn count_branches(block: &Block) -> u32 {
    block
        .children
        .iter()
        .map(|n| match n {
            Node::Branch(_) => 1,
            Node::NestedBlock(b) => count_branches(b),
            _ => 0,
        })
        .sum()
}

fn cognitive_block(
    block: &Block,
    seen_logical_sequences: &mut HashSet<(BranchKind, u32, u32)>,
) -> u32 {
    block
        .children
        .iter()
        .map(|node| cognitive_node(node, seen_logical_sequences))
        .sum()
}

/// Cognitive contribution of one block child; recurses into nested blocks.
fn cognitive_node(node: &Node, seen: &mut HashSet<(BranchKind, u32, u32)>) -> u32 {
    match node {
        Node::Branch(branch)
            if should_count_cognitive_branch(
                branch.kind,
                branch.sequence_id,
                branch.nesting_at,
                seen,
            ) =>
        {
            branch.cognitive_weight * (1 + branch.nesting_at)
        }
        Node::Branch(_) => 0,
        Node::NestedBlock(block) => cognitive_block(block, seen),
        _ => 0,
    }
}

fn should_count_cognitive_branch(
    kind: BranchKind,
    sequence_id: Option<u32>,
    nesting_at: u32,
    seen_logical_sequences: &mut HashSet<(BranchKind, u32, u32)>,
) -> bool {
    match (kind, sequence_id) {
        (BranchKind::LogicalAnd | BranchKind::LogicalOr, Some(id)) => {
            seen_logical_sequences.insert((kind, id, nesting_at))
        }
        _ => true,
    }
}

fn nesting_max_block(block: &Block) -> u32 {
    block
        .children
        .iter()
        .filter_map(|node| match node {
            Node::NestedBlock(child) => Some(nesting_max_block(child)),
            _ => None,
        })
        .max()
        .map_or(block.nesting, |child_max| block.nesting.max(child_max))
}

#[derive(Debug, Default)]
struct AbcTotals {
    assignments: f64,
    branches: f64,
    calls: f64,
}

fn abc_block(block: &Block) -> AbcTotals {
    block
        .children
        .iter()
        .fold(AbcTotals::default(), |mut totals, node| {
            match node {
                Node::Assignment(_) => totals.assignments += 1.0,
                Node::Branch(_) => totals.branches += 1.0,
                Node::Call(call) => totals.calls += call.smell_weight,
                Node::NestedBlock(child) => {
                    let child_totals = abc_block(child);
                    totals.assignments += child_totals.assignments;
                    totals.branches += child_totals.branches;
                    totals.calls += child_totals.calls;
                }
                _ => {}
            }
            totals
        })
}

#[cfg(test)]
mod tests;
