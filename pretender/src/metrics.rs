#![allow(dead_code)]

use crate::model::{Block, BranchKind, CodeUnit, Metric, Node};
use std::collections::HashSet;

pub struct CyclomaticComplexity;

impl Metric for CyclomaticComplexity {
    fn name(&self) -> &'static str {
        "Cyclomatic Complexity"
    }

    fn calculate(&self, unit: &CodeUnit) -> u32 {
        cyclomatic(unit)
    }
}

pub fn cyclomatic(unit: &CodeUnit) -> u32 {
    1 + count_branches(&unit.body)
}

pub fn cognitive(unit: &CodeUnit) -> u32 {
    let mut seen_logical_sequences = HashSet::new();
    cognitive_block(&unit.body, &mut seen_logical_sequences)
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
        .map(|node| match node {
            Node::Branch(branch)
                if should_count_cognitive_branch(
                    branch.kind,
                    branch.sequence_id,
                    branch.nesting_at,
                    seen_logical_sequences,
                ) =>
            {
                1 + branch.nesting_at
            }
            Node::Branch(_) => 0,
            Node::NestedBlock(block) => cognitive_block(block, seen_logical_sequences),
            _ => 0,
        })
        .sum()
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
mod tests {
    use super::*;
    use crate::model::{Branch, BranchKind, CallSite, Parameter, Span, UnitKind};

    fn span(start_line: u32, end_line: u32) -> Span {
        Span {
            start_line,
            end_line,
        }
    }

    fn branch(kind: BranchKind, nesting_at: u32, sequence_id: Option<u32>) -> Node {
        Node::Branch(Branch {
            kind,
            span: span(1, 1),
            nesting_at,
            sequence_id,
        })
    }

    fn unit(body: Block) -> CodeUnit {
        CodeUnit {
            name: "example".to_string(),
            kind: UnitKind::Function,
            span: span(10, 14),
            parameters: vec![
                Parameter {
                    name: "a".to_string(),
                    span: span(10, 10),
                },
                Parameter {
                    name: "b".to_string(),
                    span: span(10, 10),
                },
            ],
            body,
            is_exported: false,
        }
    }

    #[test]
    fn calculates_basic_count_metrics_recursively() {
        let code_unit = unit(Block {
            span: span(11, 13),
            nesting: 0,
            children: vec![
                branch(BranchKind::If, 0, None),
                Node::NestedBlock(Block {
                    span: span(12, 13),
                    nesting: 1,
                    children: vec![branch(BranchKind::Loop, 1, None)],
                }),
            ],
        });

        assert_eq!(cyclomatic(&code_unit), 3);
        assert_eq!(function_lines(&code_unit), 5);
        assert_eq!(params(&code_unit), 2);
        assert_eq!(nesting_max(&code_unit), 1);
    }

    #[test]
    fn cognitive_counts_logical_sequences_once_and_weights_nesting() {
        let code_unit = unit(Block {
            span: span(1, 5),
            nesting: 0,
            children: vec![
                branch(BranchKind::If, 0, None),
                branch(BranchKind::LogicalAnd, 0, Some(1)),
                branch(BranchKind::LogicalAnd, 0, Some(1)),
                branch(BranchKind::LogicalOr, 0, Some(2)),
                Node::NestedBlock(Block {
                    span: span(2, 4),
                    nesting: 1,
                    children: vec![branch(BranchKind::Loop, 1, None)],
                }),
            ],
        });

        assert_eq!(cognitive(&code_unit), 5);
    }

    #[test]
    fn empty_unit_has_baseline_metric_values() {
        let code_unit = unit(Block {
            span: span(1, 1),
            nesting: 0,
            children: Vec::new(),
        });

        assert_eq!(cyclomatic(&code_unit), 1);
        assert_eq!(cognitive(&code_unit), 0);
        assert_eq!(nesting_max(&code_unit), 0);
        assert_eq!(abc(&code_unit), 0.0);
    }

    #[test]
    fn abc_uses_assignments_branches_and_weighted_calls() {
        let code_unit = unit(Block {
            span: span(1, 3),
            nesting: 0,
            children: vec![
                Node::Assignment(span(1, 1)),
                Node::Assignment(span(2, 2)),
                branch(BranchKind::If, 0, None),
                Node::Call(CallSite {
                    callee: "expensive".to_string(),
                    span: span(3, 3),
                    smell_weight: 1.5,
                }),
                Node::NestedBlock(Block {
                    span: span(3, 3),
                    nesting: 1,
                    children: vec![Node::Call(CallSite {
                        callee: "normal".to_string(),
                        span: span(3, 3),
                        smell_weight: 1.0,
                    })],
                }),
            ],
        });

        let score = abc(&code_unit);

        assert!((score - (2.0_f64 * 2.0 + 1.0 + 2.5_f64 * 2.5).sqrt()).abs() < 0.000_001);
    }
}
