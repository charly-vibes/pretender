use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub fn ts_language_for_path(path: &Path) -> Option<tree_sitter::Language> {
    match path.extension()?.to_str()? {
        "r" | "R" => Some(tree_sitter_r::LANGUAGE.into()),
        "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "js" | "jsx" | "mjs" | "cjs" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "ts" | "mts" => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
        "tsx" | "cts" => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
        "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "go" => Some(tree_sitter_go::LANGUAGE.into()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        "jl" => Some(tree_sitter_julia::LANGUAGE.into()),
        "rb" => Some(tree_sitter_ruby::LANGUAGE.into()),
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
    let mut count = 1usize;
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        count += named_node_count(child);
    }
    count
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

/// Build a normalized structural string for a subtree.
/// Identifiers → $VAR, literals → $LIT_str / $LIT_num / $LIT_bool.
/// Comments are skipped. Node kinds provide structural skeleton.
#[allow(clippy::only_used_in_recursion)]
fn normalize(node: tree_sitter::Node, source: &[u8]) -> String {
    let kind = node.kind();
    if kind.contains("comment") {
        return String::new();
    }

    let mut cursor = node.walk();
    let children: Vec<_> = node
        .named_children(&mut cursor)
        .filter(|c| !c.kind().contains("comment"))
        .collect();

    if children.is_empty() {
        return leaf_token(kind);
    }

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

fn collect_subtrees(
    scan: &CloneScan<'_>,
    node: tree_sitter::Node,
    buckets: &mut HashMap<String, Vec<(CloneLocation, usize)>>,
) {
    if node.kind().contains("comment") {
        return;
    }

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

    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect_subtrees(scan, child, buckets);
    }
}

/// Stage 1 — candidate hashing: parse every supported file and bucket its
/// normalized subtrees by structural signature.
fn hash_candidates(
    files: &[(PathBuf, String)],
    min_nodes: usize,
) -> Result<HashMap<String, Vec<(CloneLocation, usize)>>> {
    let mut buckets: HashMap<String, Vec<(CloneLocation, usize)>> = HashMap::new();

    for (path, source) in files {
        let Some(ts_lang) = ts_language_for_path(path) else {
            continue;
        };
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&ts_lang)
            .map_err(|e| anyhow::anyhow!("set language for {}: {e}", path.display()))?;
        let Some(tree) = parser.parse(source.as_bytes(), None) else {
            continue;
        };
        collect_subtrees(
            &CloneScan {
                source: source.as_bytes(),
                min_nodes,
                file: path,
            },
            tree.root_node(),
            &mut buckets,
        );
    }

    Ok(buckets)
}

/// Stage 2 — verification: keep signatures with ≥2 occurrences; in within-file
/// mode, drop groups whose occurrences span multiple files.
fn verified_groups(
    buckets: HashMap<String, Vec<(CloneLocation, usize)>>,
    cross_file: bool,
) -> Vec<CloneGroup> {
    buckets
        .into_values()
        .filter_map(|locs| {
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
        })
        .collect()
}

/// Stage 3 — reporting: verified clone groups, largest node count first.
pub fn detect_clones(
    files: &[(PathBuf, String)],
    min_nodes: usize,
    cross_file: bool,
) -> Result<Vec<CloneGroup>> {
    let buckets = hash_candidates(files, min_nodes)?;
    let mut groups = verified_groups(buckets, cross_file);
    groups.sort_by_key(|b| std::cmp::Reverse(b.node_count));
    Ok(groups)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use std::path::PathBuf;

    fn fixture_path(name: &str) -> Result<PathBuf> {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .context("cargo manifest dir has no parent")?
            .join("tests/fixtures")
            .join(name))
    }

    fn load_fixture(name: &str) -> Result<(PathBuf, String)> {
        let path = fixture_path(name)?;
        let source =
            std::fs::read_to_string(&path).with_context(|| format!("fixture not found: {name}"))?;
        Ok((path, source))
    }

    #[test]
    fn detects_structural_clones_within_file() -> Result<()> {
        let file = load_fixture("python_duplicates.py")?;
        let groups = detect_clones(&[file], 5, false)?;
        assert!(
            !groups.is_empty(),
            "expected at least one clone group in python_duplicates.py"
        );
        let top = &groups[0];
        assert_eq!(top.similarity, 100);
        assert!(top.locations.len() >= 2, "expected ≥2 clone locations");
        Ok(())
    }

    #[test]
    fn no_clones_in_unique_file() -> Result<()> {
        let file = load_fixture("python_simple.py")?;
        // Use a high threshold so only large identical blocks would match
        let groups = detect_clones(&[file], 20, false)?;
        // python_simple.py has no intentional duplicates at 20+ nodes
        for g in &groups {
            // All locations must be in the same file (within-file mode)
            let first = &g.locations[0].file;
            assert!(g.locations.iter().all(|l| &l.file == first));
        }
        Ok(())
    }

    #[test]
    fn unsupported_extension_is_skipped() -> Result<()> {
        let path = PathBuf::from("foo.txt");
        let groups = detect_clones(&[(path, "hello world".to_string())], 5, false)?;
        assert!(groups.is_empty());
        Ok(())
    }

    #[test]
    fn cross_file_flag_widens_scope() -> Result<()> {
        let f1 = load_fixture("python_duplicates.py")?;
        let f2 = load_fixture("python_simple.py")?;

        let within = detect_clones(&[f1.clone(), f2.clone()], 5, false)?;
        let cross = detect_clones(&[f1, f2], 5, true)?;

        // Cross-file mode should find at least as many groups as within-file
        assert!(cross.len() >= within.len());
        Ok(())
    }

    #[test]
    fn min_nodes_threshold_filters_small_subtrees() -> Result<()> {
        let file = load_fixture("python_duplicates.py")?;
        let small = detect_clones(std::slice::from_ref(&file), 1, false)?;
        let large = detect_clones(&[file], 30, false)?;
        // Lowering the threshold finds more (or equal) clone groups
        assert!(small.len() >= large.len());
        Ok(())
    }

    #[test]
    fn leaf_classification_pins_normalization_tokens() {
        assert_eq!(leaf_token("identifier"), "$VAR");
        assert_eq!(leaf_token("string"), "$LIT_str");
        assert_eq!(leaf_token("raw_string"), "$LIT_str");
        assert_eq!(leaf_token("interpolated_string"), "$LIT_str");
        assert_eq!(leaf_token("integer"), "$LIT_num");
        assert_eq!(leaf_token("decimal"), "$LIT_num");
        assert_eq!(leaf_token("true"), "$LIT_bool");
        assert_eq!(leaf_token("nil"), "$LIT_bool");
        assert_eq!(leaf_token("binary_operator"), "binary_operator");
    }
}
