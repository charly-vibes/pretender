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

/// The shared 3-line span for fixture units starting at `start`.
fn unit_span(start: u32) -> Span {
    Span {
        start_line: start,
        end_line: start + 2,
    }
}

fn make_unit(name: &str, body_children: Vec<Node>, assertions: u32, start: u32) -> CodeUnit {
    CodeUnit {
        name: name.to_string(),
        kind: crate::model::UnitKind::Function,
        span: unit_span(start),
        parameters: vec![],
        body: Block {
            span: unit_span(start),
            nesting: 0,
            children: body_children,
        },
        is_exported: false,
        assertions,
        parent_class: None,
        has_test_attribute: false,
    }
}

/// One `add`-calling unit with a single `if` branch and two assertions —
/// the shape shared by the cluster fixtures below.
fn call_if_unit(name: &str, start: u32) -> CodeUnit {
    make_unit(
        name,
        vec![make_call("add"), make_branch(BranchKind::If)],
        2,
        start,
    )
}

#[test]
fn detects_cluster_of_three() {
    let units = vec![
        call_if_unit("test_add_1", 1),
        call_if_unit("test_add_2", 5),
        call_if_unit("test_add_3", 9),
    ];
    let clusters = detect(&units, 3);
    assert_eq!(clusters.len(), 1);
    assert_eq!(clusters[0].count, 3);
}

#[test]
fn different_structure_not_clustered() {
    // test_b adds a Loop branch, so its fingerprint differs from
    // test_a/test_c and no cluster of 3 forms.
    let test_b = make_unit(
        "test_b",
        vec![
            make_call("add"),
            make_branch(BranchKind::If),
            make_branch(BranchKind::Loop),
        ],
        2,
        5,
    );
    let units = vec![call_if_unit("test_a", 1), test_b, call_if_unit("test_c", 9)];
    let clusters = detect(&units, 3);
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
