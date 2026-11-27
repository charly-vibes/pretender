//! Walk and exclude-glob behavior tests for `super` (input collection).
//!
//! Moved verbatim from the single-file collect.rs (pretender-5zo.15.12) so
//! the walk implementation stays within the per-file size limit; the
//! gitignore and glob-normalization behavior they pin is unchanged.

use super::*;

// ── gitignore walk behavior (pretender-094) ──────────────────────────
mod gitignore_walk_tests {
    use super::*;
    use tempfile::TempDir;

    /// Test fixture writer: panics with the path in the message instead of
    /// unwrapping, keeping the file's unwrap count within the structural
    /// limit.
    fn fs_write(path: &std::path::Path, contents: &str) {
        std::fs::write(path, contents).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    }

    /// Temp repo fixture: git-initialized, with `target/` and `vendor/`
    /// gitignored, one source file at the root, one in each ignored dir.
    fn fixture_repo() -> (TempDir, std::path::PathBuf) {
        let dir = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
        let root = dir.path().to_path_buf();
        init_ignored_repo(&root);
        write_repo_sources(&root);
        (dir, root)
    }

    /// Git init plus the `.gitignore` and ignored directories.
    fn init_ignored_repo(root: &std::path::Path) {
        let out = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(root)
            .output()
            .unwrap_or_else(|e| panic!("git init: {e}"));
        assert!(
            out.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        fs_write(&root.join(".gitignore"), "target/\nvendor/\n");
        mkdir(root, "target");
        mkdir(root, "vendor");
    }

    /// One source file at the root, one in each ignored dir.
    fn write_repo_sources(root: &std::path::Path) {
        fs_write(&root.join("root.py"), "def f():\n    return 1\n");
        fs_write(
            &root.join("target").join("gen.py"),
            "def g():\n    return 2\n",
        );
        fs_write(
            &root.join("vendor").join("lib.py"),
            "def h():\n    return 3\n",
        );
    }

    /// Create `dir/<name>`, panicking with the name in the message.
    fn mkdir(root: &std::path::Path, name: &str) {
        std::fs::create_dir_all(root.join(name)).unwrap_or_else(|e| panic!("mkdir {name}: {e}"));
    }

    /// Relative, sorted names for readable assertions.
    fn rel_names(root: &std::path::Path, files: &[std::path::PathBuf]) -> Vec<String> {
        let mut v: Vec<String> = files
            .iter()
            .map(|p| {
                p.strip_prefix(root)
                    .unwrap_or_else(|e| panic!("strip_prefix {p:?}: {e}"))
                    .display()
                    .to_string()
            })
            .collect();
        v.sort();
        v
    }

    #[test]
    fn walk_skips_gitignored_dirs_by_default() {
        let (_dir, root) = fixture_repo();
        let config = Config::default();
        assert!(
            config.pretender.respect_gitignore,
            "default must honor .gitignore"
        );

        let files = collect_input_files(std::slice::from_ref(&root), &config)
            .unwrap_or_else(|e| panic!("collect: {e}"));
        assert_eq!(
            rel_names(&root, &files),
            vec!["root.py".to_string()],
            "gitignored target/ and vendor/ must not be scanned"
        );
    }

    #[test]
    fn opt_out_restores_exhaustive_walk() {
        let (_dir, root) = fixture_repo();
        let mut config = Config::default();
        config.pretender.respect_gitignore = false;

        let files = collect_input_files(std::slice::from_ref(&root), &config)
            .unwrap_or_else(|e| panic!("collect: {e}"));
        assert_eq!(
            rel_names(&root, &files),
            vec![
                "root.py".to_string(),
                "target/gen.py".to_string(),
                "vendor/lib.py".to_string(),
            ]
        );
    }

    #[test]
    fn explicit_ignored_file_still_scanned() {
        let (_dir, root) = fixture_repo();
        let config = Config::default();
        let ignored = root.join("vendor").join("lib.py");

        let files = collect_input_files(std::slice::from_ref(&ignored), &config)
            .unwrap_or_else(|e| panic!("collect: {e}"));
        assert_eq!(
            files.len(),
            1,
            "explicitly passed paths bypass the gitignore filter"
        );
    }

    #[test]
    fn no_gitignore_falls_back_to_full_walk() {
        let dir = TempDir::new().unwrap_or_else(|e| panic!("tempdir: {e}"));
        let root = dir.path().to_path_buf();
        fs_write(&root.join("a.py"), "def f():\n    return 1\n");
        fs_write(&root.join("b.py"), "def g():\n    return 2\n");

        let config = Config::default();
        let files = collect_input_files(std::slice::from_ref(&root), &config)
            .unwrap_or_else(|e| panic!("collect: {e}"));
        assert_eq!(
            rel_names(&root, &files).len(),
            2,
            "no .gitignore: nothing filtered"
        );
    }
}

// ── exclude glob normalization (espectacular-dwc) ────────────────────
mod exclude_glob_tests {
    use super::*;

    fn exclusions(patterns: &[&str]) -> globset::GlobSet {
        let owned: Vec<String> = patterns.iter().map(|p| p.to_string()).collect();
        build_exclude_set(&owned).unwrap_or_else(|e| panic!("build exclude set: {e}"))
    }

    #[test]
    fn relative_patterns_match_dot_prefixed_walked_paths() {
        let set = exclusions(&["tests/**", "vendor/**"]);
        assert!(is_excluded(&set, Path::new("./tests/test_a.py")));
        assert!(is_excluded(&set, Path::new("./vendor/lib.py")));
        assert!(!is_excluded(&set, Path::new("./src/app.py")));
    }

    #[test]
    fn dot_prefixed_pattern_matches_plain_walked_path() {
        let set = exclusions(&["./tests/**"]);
        assert!(is_excluded(&set, Path::new("./tests/test_a.py")));
        assert!(is_excluded(&set, Path::new("tests/test_a.py")));
        assert!(!is_excluded(&set, Path::new("./vendor/lib.py")));
    }

    #[test]
    fn absolute_paths_keep_prior_behavior() {
        let set = exclusions(&["tests/**"]);
        assert!(!is_excluded(&set, Path::new("/repo/tests/test_a.py")));
    }
}
