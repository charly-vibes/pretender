//! Body-walk machinery for the query engine: turns tree-sitter capture
//! hits into the weighted `Node` tree each `CodeUnit` carries.
//!
//! # Responsibilities
//! [`build_block`] and its recursive visitors ([`visit_child`],
//! [`walk_block`], [`collect_nested_blocks`]) assemble branches,
//! assignments, calls, and nested blocks; [`count_captured_nodes`] and
//! [`has_test_marker`] answer the two sibling questions the metric
//! layer asks; [`is_nested_definition`] keeps inner definitions out of
//! their parents' bodies.
//!
//! # Rationale
//! Split from the former monolithic `engine.rs` (ratcheted in
//! pretender-5zo.15.3) along the query-vs-walk seam; the former
//! three-deep `collect_nested_blocks` recursion is decomposed into a
//! sibling-descents helper so nesting stays within the target cap,
//! with traversal order and output byte-identical.

use std::collections::BTreeMap;

use crate::metric_application::{BranchCaptureSpec, CaptureMap};
use crate::model::{Block, Branch, BranchKind, CallSite, Span};

/// Shared immutable context threaded through the block walk below:
/// everything the recursive visitors need except the mutable output list.
#[derive(Clone, Copy)]
pub(crate) struct WalkCtx<'a> {
    pub(crate) captures: &'a CaptureMap,
    pub(crate) call_weights: &'a BTreeMap<String, f64>,
    pub(crate) nesting: u32,
}

pub(crate) fn node_span(node: tree_sitter::Node) -> Span {
    Span {
        start_line: node.start_position().row as u32 + 1,
        end_line: node.end_position().row as u32 + 1,
    }
}

pub(crate) fn build_block(ctx: &WalkCtx, block_node: tree_sitter::Node) -> Block {
    let mut children = Vec::new();
    // Check the block_node itself against captures first.
    // This handles languages where the function body IS a branch node
    // (e.g. Clojure: (defn f [x] (if ...)) where the entire body is an if form).
    if !visit_child(ctx, block_node, block_node, &mut children) {
        walk_block(ctx, block_node, &mut children);
    }
    Block {
        span: node_span(block_node),
        nesting: ctx.nesting,
        children,
    }
}

fn visit_child(
    ctx: &WalkCtx,
    parent: tree_sitter::Node,
    child: tree_sitter::Node,
    out: &mut Vec<crate::model::Node>,
) -> bool {
    let Some(node) = captured_node(ctx, parent, child) else {
        return false;
    };
    out.push(node);
    collect_nested_blocks(ctx, child, out);
    true
}

/// The captured model node for `child`, if any capture claims it:
/// a weighted branch, an assignment, or a call site, in that order.
fn captured_node(
    ctx: &WalkCtx,
    parent: tree_sitter::Node,
    child: tree_sitter::Node,
) -> Option<crate::model::Node> {
    if let Some(&spec) = ctx.captures.branches.get(&child.id()) {
        return Some(crate::model::Node::Branch(branch_node(
            ctx, parent, child, spec,
        )));
    }
    if ctx.captures.assignments.contains(&child.id()) {
        return Some(crate::model::Node::Assignment(node_span(child)));
    }
    ctx.captures
        .calls
        .get(&child.id())
        .map(|callee| crate::model::Node::Call(call_site(ctx, child, callee)))
}

/// A weighted branch node; logical operators share the parent's sequence
/// id so `a and b and c` reports as one sequence.
fn branch_node(
    ctx: &WalkCtx,
    parent: tree_sitter::Node,
    child: tree_sitter::Node,
    spec: BranchCaptureSpec,
) -> Branch {
    let sequence_id = match spec.kind {
        BranchKind::LogicalAnd | BranchKind::LogicalOr => Some(parent.id() as u32),
        _ => None,
    };
    Branch {
        kind: spec.kind,
        span: node_span(child),
        nesting_at: ctx.nesting,
        sequence_id,
        cyclomatic_weight: spec.cyclomatic_weight,
        cognitive_weight: spec.cognitive_weight,
    }
}

/// A call site with the adapter's smell weight for the callee, or 1.0.
fn call_site(ctx: &WalkCtx, child: tree_sitter::Node, callee: &str) -> CallSite {
    let smell_weight = ctx.call_weights.get(callee).copied().unwrap_or(1.0);
    CallSite {
        callee: callee.to_owned(),
        span: node_span(child),
        smell_weight,
    }
}

fn walk_block(ctx: &WalkCtx, node: tree_sitter::Node, out: &mut Vec<crate::model::Node>) {
    for child in node.children(&mut node.walk()) {
        if is_nested_definition(child) {
            continue;
        }
        if !visit_child(ctx, node, child, out) {
            walk_block(ctx, child, out);
        }
    }
}

fn collect_nested_blocks(
    ctx: &WalkCtx,
    branch_node: tree_sitter::Node,
    out: &mut Vec<crate::model::Node>,
) {
    for child in branch_node.children(&mut branch_node.walk()) {
        if is_nested_definition(child) {
            continue;
        }
        descend_unvisited(ctx, branch_node, child, out);
    }
}

/// A branch child no capture claimed: a `block` node recurses one
/// nesting level deeper as a [`crate::model::Node::NestedBlock`]; any
/// other uncaptured container keeps collecting at this level.
fn descend_unvisited(
    ctx: &WalkCtx,
    parent: tree_sitter::Node,
    child: tree_sitter::Node,
    out: &mut Vec<crate::model::Node>,
) {
    if visit_child(ctx, parent, child, out) {
        return;
    }
    if child.kind() == "block" {
        let inner = WalkCtx {
            nesting: ctx.nesting + 1,
            ..*ctx
        };
        out.push(crate::model::Node::NestedBlock(build_block(&inner, child)));
    } else {
        collect_nested_blocks(ctx, child, out);
    }
}

pub(crate) fn count_captured_nodes(
    node: tree_sitter::Node,
    captured_ids: &std::collections::HashSet<usize>,
) -> u32 {
    u32::from(captured_ids.contains(&node.id()))
        + node
            .children(&mut node.walk())
            .filter(|child| !is_nested_definition(*child))
            .map(|child| count_captured_nodes(child, captured_ids))
            .sum::<u32>()
}

/// True when the definition carries a test-registration attribute
/// (e.g. Rust `#[test]`). Attributes precede the definition as siblings;
/// scanning stops at the first non-attribute sibling so an outer unit
/// never claims another unit's marker.
pub(crate) fn has_test_marker(def_node: tree_sitter::Node, source: &[u8]) -> bool {
    std::iter::successors(def_node.prev_sibling(), |node| node.prev_sibling())
        .take_while(|node| node.kind() == "attribute_item")
        .any(|node| {
            node.utf8_text(source)
                .map(|text| text.contains("test"))
                .unwrap_or(false)
        })
}

fn is_nested_definition(node: tree_sitter::Node) -> bool {
    matches!(
        node.kind(),
        "function_definition"
            | "class_definition"
            | "function_declaration"
            | "method_definition"
            | "method_declaration"
            | "generator_function_declaration"
            | "arrow_function"
            | "function_item"
            | "class"
            // Anonymous function literals / closures across languages
            | "func_literal"           // Go
            | "closure_expression"     // Rust
            | "lambda"                 // Python
            | "lambda_expression"       // Java, C++, C#
            | "anonymous_method_expression" // C#
    )
}
