use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Subtree buckets keyed by normalized structural signature.
type Buckets = HashMap<String, Vec<(CloneLocation, usize)>>;

pub fn ts_language_for_path(path: &Path) -> Option<tree_sitter::Language> {
    let ext = path.extension()?.to_str()?;
    script_language(ext).or_else(|| compiled_language(ext))
}

/// tree-sitter language for scripting-language extensions.
fn script_language(ext: &str) -> Option<tree_sitter::Language> {
    match ext {
        "r" | "R" => Some(tree_sitter_r::LANGUAGE.into()),
        "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "js" | "jsx" | "mjs" | "cjs" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "rb" => Some(tree_sitter_ruby::LANGUAGE.into()),
        "jl" => Some(tree_sitter_julia::LANGUAGE.into()),
        _ => None,
    }
}

/// tree-sitter language for compiled/statically-typed-language extensions.
fn compiled_language(ext: &str) -> Option<tree_sitter::Language> {
    match ext {
        "ts" | "mts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" | "cts" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        "c" | "h" => Some(tree_sitter_c::LANGUAGE.into()),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(tree_sitter_cpp::LANGUAGE.into()),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub struct CloneLocation {
    pub file: PathBuf,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Debug)]
pub struct CloneGroup {
    pub node_count: usize,
    pub similarity: u32,
    pub locations: Vec<CloneLocation>,
}

/// Count named nodes in a subtree (excludes whitespace/comment nodes used as threshold).
fn named_node_count(node: tree_sitter::Node) -> usize {
    let mut cursor = node.walk();
    1 + node
        .named_children(&mut cursor)
        .map(named_node_count)
        .sum::<usize>()
}

/// Classify a leaf node kind into its normalized structural token.
/// Leaves carry no child structure, so kinds collapse into $VAR / $LIT_* tokens
/// (or fall back to their own kind when no mapping applies).
fn leaf_token(kind: &str) -> String {
    if kind.contains("identifier") {
        return "$VAR".to_string();
    }
    if kind.contains("string") || kind == "raw_string" || kind == "interpolated_string" {
        return "$LIT_str".to_string();
    }
    if kind.contains("integer")
        || kind.contains("float")
        || kind.contains("number")
        || kind == "decimal"
    {
        return "$LIT_num".to_string();
    }
    if matches!(kind, "true" | "false" | "None" | "nil" | "null") {
        return "$LIT_bool".to_string();
    }
    kind.to_string()
}

/// Non-comment named children of a node, gathered once per parent.
fn named_non_comments<'a>(node: tree_sitter::Node<'a>) -> Vec<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|c| !c.kind().contains("comment"))
        .collect()
}

/// Build a normalized structural string for a subtree.
/// Identifiers → $VAR, literals → $LIT_str / $LIT_num / $LIT_bool.
/// Comments are skipped. Node kinds provide structural skeleton.
#[allow(clippy::only_used_in_recursion)]
fn normalize(node: tree_sitter::Node, source: &[u8]) -> String {
    let kind = node.kind();
    if kind.contains("comment") {
        return String::new();
    }

    let children = named_non_comments(node);

    if children.is_empty() {
        return leaf_token(kind);
    }

    normalized_children(kind, &children, source)
}

/// `kind(child1,child2,…)` over the normalized forms of non-empty children.
fn normalized_children(kind: &str, children: &[tree_sitter::Node<'_>], source: &[u8]) -> String {
    let parts: Vec<String> = children
        .iter()
        .map(|c| normalize(*c, source))
        .filter(|s| !s.is_empty())
        .collect();

    format!("{}({})", kind, parts.join(","))
}

/// Per-file scan configuration for `collect_subtrees`.
struct CloneScan<'a> {
    source: &'a [u8],
    min_nodes: usize,
    file: &'a Path,
}

fn collect_subtrees(scan: &CloneScan<'_>, node: tree_sitter::Node, buckets: &mut Buckets) {
    if node.kind().contains("comment") {
        return;
    }

    record_subtree(scan, node, buckets);

    for child in named_non_comments(node) {
        collect_subtrees(scan, child, buckets);
    }
}

/// Bucket one subtree when it meets the node-count threshold and
/// normalizes to a non-empty signature.
fn record_subtree(scan: &CloneScan<'_>, node: tree_sitter::Node, buckets: &mut Buckets) {
    let count = named_node_count(node);
    if count >= scan.min_nodes {
        let norm = normalize(node, scan.source);
        if !norm.is_empty() {
            let loc = CloneLocation {
                file: scan.file.to_path_buf(),
                start_line: node.start_position().row as u32 + 1,
                end_line: node.end_position().row as u32 + 1,
            };
            buckets.entry(norm).or_default().push((loc, count));
        }
    }
}

/// Stage 1 — candidate hashing: parse every supported file and bucket its
/// normalized subtrees by structural signature.
fn hash_candidates(files: &[(PathBuf, String)], min_nodes: usize) -> Result<Buckets> {
    let mut buckets: Buckets = HashMap::new();

    for (path, source) in files {
        hash_file(path, source, min_nodes, &mut buckets)?;
    }

    Ok(buckets)
}

/// Parse one file and bucket its normalized subtrees; unsupported
/// extensions and unparseable sources are skipped silently.
fn hash_file(path: &Path, source: &str, min_nodes: usize, buckets: &mut Buckets) -> Result<()> {
    let Some(ts_lang) = ts_language_for_path(path) else {
        return Ok(());
    };
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&ts_lang)
        .map_err(|e| anyhow::anyhow!("set language for {}: {e}", path.display()))?;
    let Some(tree) = parser.parse(source.as_bytes(), None) else {
        return Ok(());
    };
    let scan = CloneScan {
        source: source.as_bytes(),
        min_nodes,
        file: path,
    };
    collect_subtrees(&scan, tree.root_node(), buckets);
    Ok(())
}

/// Stage 2 — verification: keep signatures with ≥2 occurrences; in within-file
/// mode, drop groups whose occurrences span multiple files.
fn verified_groups(
    buckets: HashMap<String, Vec<(CloneLocation, usize)>>,
    cross_file: bool,
) -> Vec<CloneGroup> {
    buckets
        .into_values()
        .filter_map(|locs| verified_group(locs, cross_file))
        .collect()
}

/// The clone group for one signature's occurrences, or None when it has
/// fewer than 2 occurrences or (in within-file mode) spans multiple files.
fn verified_group(locs: Vec<(CloneLocation, usize)>, cross_file: bool) -> Option<CloneGroup> {
    if locs.len() < 2 {
        return None;
    }
    if !cross_file {
        let first = &locs[0].0.file;
        if !locs.iter().all(|(l, _)| l.file == *first) {
            return None;
        }
    }
    let node_count = locs[0].1;
    let locations = locs.into_iter().map(|(l, _)| l).collect();
    Some(CloneGroup {
        node_count,
        similarity: 100,
        locations,
    })
}

/// Stage 3 — reporting: verified clone groups, largest node count first.
pub fn detect_clones(
    files: &[(PathBuf, String)],
    min_nodes: usize,
    cross_file: bool,
) -> Result<Vec<CloneGroup>> {
    let buckets = hash_candidates(files, min_nodes)?;
    Ok(sort_by_size(verified_groups(buckets, cross_file)))
}

/// Order groups by node count descending, largest first.
fn sort_by_size(mut groups: Vec<CloneGroup>) -> Vec<CloneGroup> {
    groups.sort_by_key(|b| std::cmp::Reverse(b.node_count));
    groups
}

#[cfg(test)]
mod tests;
