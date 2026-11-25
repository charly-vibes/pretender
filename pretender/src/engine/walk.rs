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

use super::clause::{clause_or_branch_nodes, named_forms};
use crate::metric_application::CaptureMap;
use crate::model::{Block, CallSite, Span};

/// Shared immutable context threaded through the block walk below:
/// everything the recursive visitors need except the mutable output list.
#[derive(Clone, Copy)]
pub(crate) struct WalkCtx<'a> {
    pub(crate) captures: &'a CaptureMap,
    pub(crate) call_weights: &'a BTreeMap<String, f64>,
    /// D4: branch-bearing forms wrap their body one level deeper.
    pub(crate) wrap_branch_bodies: bool,
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
    let Some(nodes) = captured_node(ctx, parent, child) else {
        return false;
    };
    out.extend(nodes);
    if let Some(&spec) = ctx.captures.branches.get(&child.id()) {
        if ctx.wrap_branch_bodies && spec.kind.nests_body() {
            wrap_branch_body(ctx, child, out);
            return true;
        }
    }
    collect_nested_blocks(ctx, child, out);
    true
}

/// D4: the branch-bearing form's body forms (its named children after
/// the leading symbol) wrap one level deeper as a single
/// [`crate::model::Node::NestedBlock`], mirroring Python's grammar-level
/// `block` nodes. The adapter opts in via
/// `[nesting] wrap_branch_bodies`; consumed forms (let/do/...) never
/// take this path and stay at the same level.
fn wrap_branch_body(ctx: &WalkCtx, form: tree_sitter::Node, out: &mut Vec<crate::model::Node>) {
    let forms = named_forms(form);
    let body = &forms[usize::min(1, forms.len())..];
    if body.is_empty() {
        return;
    }
    let inner = WalkCtx {
        nesting: ctx.nesting + 1,
        ..*ctx
    };
    let children = visit_body_forms(&inner, form, body);
    out.push(crate::model::Node::NestedBlock(crate::model::Block {
        span: node_span(form),
        nesting: inner.nesting,
        children,
    }));
}

/// `form`'s body forms visited one level deeper, in traversal order.
fn visit_body_forms(
    inner: &WalkCtx,
    form: tree_sitter::Node,
    body: &[tree_sitter::Node],
) -> Vec<crate::model::Node> {
    let mut children = Vec::new();
    for f in body {
        if !visit_child(inner, form, *f, &mut children) {
            walk_block(inner, *f, &mut children);
        }
    }
    children
}

/// The captured model nodes for `child`, if any capture claims it:
/// a neutral consumed form (claimed, emits nothing), clause branches
/// (one per test/result pair), a weighted branch, an assignment, or a
/// call site — in that order (D3 capture precedence).
fn captured_node(
    ctx: &WalkCtx,
    parent: tree_sitter::Node,
    child: tree_sitter::Node,
) -> Option<Vec<crate::model::Node>> {
    if ctx.captures.consumed.contains(&child.id()) {
        return Some(Vec::new());
    }
    if let Some(&spec) = ctx.captures.branches.get(&child.id()) {
        return Some(clause_or_branch_nodes(ctx, parent, child, spec));
    }
    if ctx.captures.assignments.contains(&child.id()) {
        return Some(vec![crate::model::Node::Assignment(node_span(child))]);
    }
    ctx.captures
        .calls
        .get(&child.id())
        .map(|callee| vec![crate::model::Node::Call(call_site(ctx, child, callee))])
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
