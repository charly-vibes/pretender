//! Line classification for parsed files: total/code/comment tallies.
//!
//! Purpose: one pass over the concrete-syntax tree decides which physical
//! lines carry code and which carry comments, so `Module` line metrics come
//! from a single authoritative tally.
//! Responsibilities: `LineCounts::collect` walks the tree once and returns
//! the tallies; the classify helpers mark leaf spans, treating comment-node
//! lines as comment unless code also touches them.
//! Rationale: split verbatim out of `parse.rs` (pretender-5zo.2 container)
//! so the parse pipeline module stays under the file cap and classification
//! owns its own seam.

use std::collections::HashSet;

/// Classified line counts for one file (total, code, comment).
pub(crate) struct LineCounts {
    pub(crate) total: u32,
    pub(crate) code: u32,
    pub(crate) comment: u32,
}

impl LineCounts {
    pub(crate) fn collect(root: tree_sitter::Node, source: &str) -> Self {
        let lines_total = source.lines().count() as u32;
        let (code, comment) = classify_lines(root, lines_total);
        Self {
            total: lines_total,
            code,
            comment,
        }
    }
}

/// Classify each line in the file using tree-sitter node types.
/// Returns (lines_code, lines_comment).
/// A line is a comment line if every leaf node on that line is a comment.
/// Otherwise if any named non-comment leaf touches the line, it is a code line.
fn classify_lines(root: tree_sitter::Node, lines_total: u32) -> (u32, u32) {
    let (code, comment) = walk_classify(HashSet::new(), HashSet::new(), root);
    (
        code.len() as u32,
        comment.iter().filter(|&&l| l <= lines_total).count() as u32,
    )
}

/// Depth-first walk over the whole tree, classifying every leaf's lines.
fn walk_classify(
    mut code: HashSet<u32>,
    mut comment: HashSet<u32>,
    root: tree_sitter::Node,
) -> (HashSet<u32>, HashSet<u32>) {
    let mut cursor = root.walk();
    loop {
        classify_leaf(cursor.node(), &mut code, &mut comment);
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return (code, comment);
            }
        }
    }
}

/// Apply one leaf node's classification to every line it touches.
fn classify_leaf(node: tree_sitter::Node, code: &mut HashSet<u32>, comment: &mut HashSet<u32>) {
    if node.child_count() != 0 {
        return;
    }
    let start = node.start_position().row as u32 + 1;
    let end = node.end_position().row as u32 + 1;
    let is_comment = node.kind().contains("comment");
    for line in start..=end {
        if is_comment {
            if !code.contains(&line) {
                comment.insert(line);
            }
        } else if !node.kind().trim().is_empty() {
            code.insert(line);
            comment.remove(&line);
        }
    }
}
