use super::*;
use crate::model::{Branch, BranchKind, CallSite, Parameter, Span, UnitKind};

fn span(start_line: u32, end_line: u32) -> Span {
    Span {
        start_line,
        end_line,
    }
}

fn branch(kind: BranchKind, nesting_at: u32, sequence_id: Option<u32>) -> Node {
    weighted_branch(kind, nesting_at, sequence_id, (1, 1))
}

fn weighted_branch(
    kind: BranchKind,
    nesting_at: u32,
    sequence_id: Option<u32>,
    weights: (u32, u32),
) -> Node {
    Node::Branch(Branch {
        kind,
        span: span(1, 1),
        nesting_at,
        sequence_id,
        cyclomatic_weight: weights.0,
        cognitive_weight: weights.1,
    })
}

fn unit(body: Block) -> CodeUnit {
    CodeUnit {
        name: "example".to_string(),
        kind: UnitKind::Function,
        span: span(10, 14),
        parameters: example_parameters(),
        body,
        parent_class: None,
        has_test_attribute: false,
        is_exported: false,
        assertions: 0,
    }
}

/// The two example parameters every test unit carries.
fn example_parameters() -> Vec<Parameter> {
    vec![
        Parameter {
            name: "a".to_string(),
            span: span(10, 10),
            type_name: None,
        },
        Parameter {
            name: "b".to_string(),
            span: span(10, 10),
            type_name: None,
        },
    ]
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
fn cognitive_uses_branch_specific_weights() {
    let code_unit = unit(Block {
        span: span(1, 3),
        nesting: 0,
        children: vec![
            weighted_branch(BranchKind::If, 0, None, (1, 3)),
            Node::NestedBlock(Block {
                span: span(2, 3),
                nesting: 1,
                children: vec![weighted_branch(BranchKind::Loop, 1, None, (1, 2))],
            }),
        ],
    });

    assert_eq!(cognitive(&code_unit), 7);
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
    let code_unit = unit(abc_fixture_block());

    let score = abc(&code_unit);

    assert!((score - (2.0_f64 * 2.0 + 1.0 + 2.5_f64 * 2.5).sqrt()).abs() < 0.000_001);
}

/// Two assignments, one branch, and weighted calls (one nested) — the
/// inputs the abc formula weights: A=2, B=1, C=1.5+1.0.
fn abc_fixture_block() -> Block {
    Block {
        span: span(1, 3),
        nesting: 0,
        children: vec![
            Node::Assignment(span(1, 1)),
            Node::Assignment(span(2, 2)),
            branch(BranchKind::If, 0, None),
            abc_call("expensive", 1.5),
            Node::NestedBlock(Block {
                span: span(3, 3),
                nesting: 1,
                children: vec![abc_call("normal", 1.0)],
            }),
        ],
    }
}

/// A call node at line 3 with the given callee and smell weight.
fn abc_call(callee: &str, smell_weight: f64) -> Node {
    Node::Call(CallSite {
        callee: callee.to_string(),
        span: span(3, 3),
        smell_weight,
    })
}
