//! Dry-run mutation-site enumeration via tree-sitter.
//!
//! Purpose: enumerate likely operator/return mutation sites in source — the
//! same sites external mutation tools typically target.
//! Responsibilities: language dispatch by extension, syntax-tree walk in
//! pre-order, operator-child lookup.
//! Rationale: split out of `mutation_entry.rs` along the report-parser /
//! source-site seam; walk order and site descriptions stay byte-identical.

use std::path::Path;

use super::PlannedMutant;

fn ts_lang_for_mutation(path: &Path) -> Option<tree_sitter::Language> {
    match path.extension()?.to_str()? {
        "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "js" | "jsx" | "mjs" | "cjs" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "ts" | "mts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" | "cts" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        _ => None,
    }
}

/// Enumerate likely mutation sites in a source file using tree-sitter.
/// Finds binary operators, comparison operators, boolean operators, and return
/// statements — the same sites that mutation tools typically target.
pub(crate) fn enumerate_mutation_sites(path: &Path, source: &str) -> Vec<PlannedMutant> {
    let Some(ts_lang) = ts_lang_for_mutation(path) else {
        return vec![];
    };
    let mut parser = tree_sitter::Parser::new();
    if parser.set_language(&ts_lang).is_err() {
        return vec![];
    }
    let Some(tree) = parser.parse(source.as_bytes(), None) else {
        return vec![];
    };
    let file_str = path.to_string_lossy().into_owned();
    let mut sites = collect_sites(tree.root_node(), source.as_bytes(), &file_str);
    sites.sort_by_key(|s| s.line);
    sites
}

/// The mutation site a single node represents, if any.
fn node_mutation_site(node: tree_sitter::Node, source: &[u8], file: &str) -> Option<PlannedMutant> {
    let line = node.start_position().row as u32 + 1;
    match node.kind() {
        // Binary arithmetic / comparison / boolean operators
        "binary_operator"
        | "comparison_operator"
        | "boolean_operator"
        | "binary_expression"
        | "augmented_assignment" => binary_op_site(node, source, file, line),
        "return_statement" | "return" => Some(PlannedMutant {
            file: file.to_string(),
            line,
            description: "replace return value".to_string(),
        }),
        _ => None,
    }
}

/// Finds the operator child of a binary node and plans its replacement.
fn binary_op_site(
    node: tree_sitter::Node,
    source: &[u8],
    file: &str,
    line: u32,
) -> Option<PlannedMutant> {
    let op_text = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .filter(|child| is_mutation_operator(child.kind()))
        .find_map(|child| child.utf8_text(source).ok())?;
    Some(PlannedMutant {
        file: file.to_string(),
        line,
        description: format!("replace operator `{op_text}`"),
    })
}

/// Depth-first pre-order collection: the node's own site, then its children.
fn collect_sites(node: tree_sitter::Node, source: &[u8], file: &str) -> Vec<PlannedMutant> {
    let own = node_mutation_site(node, source, file);
    let children = (0..node.child_count())
        .filter_map(|i| node.child(i))
        .filter(|child| child.is_named())
        .flat_map(|child| collect_sites(child, source, file));
    own.into_iter().chain(children).collect()
}

fn is_mutation_operator(kind: &str) -> bool {
    const OPS: &[&str] = &[
        "+", "-", "*", "/", "%", "**", "==", "!=", "<", ">", "<=", ">=", "and", "or", "&&", "||",
        "not", "<<", ">>", "&", "|", "^", "+=", "-=", "*=", "/=",
    ];
    OPS.contains(&kind)
}
