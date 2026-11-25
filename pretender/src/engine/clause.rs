//! Clause-kind branch expansion for the body walk: `cond`, `case`, and
//! `condp` forms yield one branch per complete test/result pair.
//!
//! # Responsibilities
//! Route clause kinds versus single branches ([`clause_or_branch_nodes`]),
//! slice a clause form into complete pairs after its dispatch head, and
//! construct the resulting weighted [`Branch`] nodes — including the
//! single-branch fallback whose logical operators share the parent's
//! sequence id.
//!
//! # Rationale
//! Split from `walk.rs` (ratcheted in pretender-5zo.15: clause helpers
//! pushed the module past the app `file_lines` cap) along the
//! traversal-vs-clause-semantics seam, with bodies verbatim so
//! traversal order and output stay byte-identical.

use crate::engine::walk::{node_span, WalkCtx};
use crate::metric_application::BranchCaptureSpec;
use crate::model::{Branch, BranchKind, Node};

/// Clause kinds emit one branch per test/result pair; every other kind
/// emits its single branch node.
pub(crate) fn clause_or_branch_nodes(
    ctx: &WalkCtx,
    parent: tree_sitter::Node,
    child: tree_sitter::Node,
    spec: BranchCaptureSpec,
) -> Vec<Node> {
    match spec.kind {
        BranchKind::CondClause | BranchKind::SwitchCase => clause_branch_nodes(ctx, child, spec),
        _ => vec![Node::Branch(branch_node(ctx, parent, child, spec))],
    }
}

/// One branch per test/result pair: pairs start after the form's head
/// (leading symbol plus any captured dispatch heads) and the test sits
/// at every second named form from there.
fn clause_branch_nodes(
    ctx: &WalkCtx,
    form: tree_sitter::Node,
    spec: BranchCaptureSpec,
) -> Vec<Node> {
    let forms = named_forms(form);
    let head = clause_head(ctx, &forms).min(forms.len());
    // chunks drop a dangling trailing expression: a clause needs
    // both a test and a result (D7 — `(case e a 1 b 2 3)` has 2 pairs).
    forms[head..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| Node::Branch(clause_branch(ctx, pair[0], spec)))
        .collect()
}

/// Named forms of a clause form; comments are excluded because they
/// never carry clause semantics in any grammar.
pub(crate) fn named_forms(form: tree_sitter::Node) -> Vec<tree_sitter::Node> {
    (0..form.named_child_count())
        .filter_map(|i| form.named_child(i))
        .filter(|c| c.kind() != "comment")
        .collect()
}

/// Head length: the leading symbol plus the form's dispatch-head
/// children (`@clause.dispatch`) — `case`'s dispatch expression, or
/// `condp`'s predicate and value.
fn clause_head(ctx: &WalkCtx, forms: &[tree_sitter::Node]) -> usize {
    1 + forms
        .iter()
        .filter(|c| ctx.captures.dispatch_heads.contains(&c.id()))
        .count()
}

fn clause_branch(ctx: &WalkCtx, test: tree_sitter::Node, spec: BranchCaptureSpec) -> Branch {
    Branch {
        kind: spec.kind,
        span: node_span(test),
        nesting_at: ctx.nesting,
        sequence_id: None,
        cyclomatic_weight: spec.cyclomatic_weight,
        cognitive_weight: spec.cognitive_weight,
    }
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
