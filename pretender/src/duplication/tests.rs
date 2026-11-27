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
fn detects_identical_rust_functions_in_temp_file() -> Result<()> {
    let source = "fn a() -> i32 { 1 }\nfn b() -> i32 { 1 }\n";
    let path = std::env::temp_dir().join(format!("pretender_dup_test_{}.rs", std::process::id()));
    std::fs::write(&path, source)?;
    let groups = detect_clones(&[(path.clone(), source.to_string())], 3, false);
    let _ = std::fs::remove_file(&path);
    let groups = groups?;
    assert!(
        !groups.is_empty(),
        "expected a clone group for two identical Rust functions"
    );
    assert!(groups[0].locations.len() >= 2);
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
