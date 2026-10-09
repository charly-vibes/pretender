use anyhow::{Context, Result};
use genesis::git::{self as ggit, EnvPolicy, GitError};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Diff-filter selecting the historical delta set: Added, Modified,
/// Renamed, Copied, Typechange — deleted files excluded.
const DIFF_FILTER: &str = "AMRCT";

/// Returns canonical absolute paths of files currently staged in the git index.
/// Deleted files are excluded — only Added, Modified, Renamed, Copied, Typechange.
pub fn staged_files(cwd: &Path) -> Result<HashSet<PathBuf>> {
    let root = repo_root(cwd)?;
    // Index vs HEAD. In an unborn-HEAD repo `git diff --cached HEAD` exits
    // 128; `git diff --cached` then diffs the index against the empty tree,
    // matching the historical `head_tree() == None` behavior exactly.
    let out = match ggit::run(
        &root,
        EnvPolicy::StripHookContext,
        &[
            "diff",
            "-z",
            "--name-only",
            "--cached",
            "--diff-filter",
            DIFF_FILTER,
            "HEAD",
        ],
    ) {
        Ok(out) => out.stdout,
        Err(GitError::Git {
            code: 128,
            ref stderr,
            ..
        }) if is_unborn_head(stderr) => {
            ggit::run(
                &root,
                EnvPolicy::StripHookContext,
                &[
                    "diff",
                    "-z",
                    "--name-only",
                    "--cached",
                    "--diff-filter",
                    DIFF_FILTER,
                ],
            )
            .context("failed to diff index against the empty tree (unborn HEAD)")?
            .stdout
        }
        Err(e) => return Err(e).context("failed to diff index against HEAD"),
    };
    Ok(paths_from_stdout(&root, &out))
}

/// Returns canonical absolute paths of files changed between `base_ref` and HEAD.
pub fn diff_base_files(cwd: &Path, base_ref: &str) -> Result<HashSet<PathBuf>> {
    let root = repo_root(cwd)?;
    let out = ggit::run(
        &root,
        EnvPolicy::StripHookContext,
        &[
            "diff",
            "-z",
            "--name-only",
            "--diff-filter",
            DIFF_FILTER,
            base_ref,
            "HEAD",
        ],
    )
    .with_context(|| format!("failed to diff '{base_ref}' against HEAD"))?;
    Ok(paths_from_stdout(&root, &out.stdout))
}

fn repo_root(cwd: &Path) -> Result<PathBuf> {
    ggit::repo_root_from(cwd).context("failed to open git repository (is this a git repo?)")
}

/// Whether a 128-exit stderr indicates HEAD does not resolve (unborn HEAD
/// in a repository without commits). Mirrors the genesis::git predicate.
fn is_unborn_head(stderr: &str) -> bool {
    stderr.contains("unknown revision") || stderr.contains("bad revision")
}

/// Parse `-z` diff output (NUL-separated, unquoted paths) into canonical
/// absolute paths under the repository root.
fn paths_from_stdout(root: &Path, stdout: &[u8]) -> HashSet<PathBuf> {
    stdout
        .split(|byte| *byte == 0)
        .filter(|raw| !raw.is_empty())
        .map(|raw| {
            let rel = String::from_utf8_lossy(raw);
            let abs = root.join(rel.as_ref());
            std::fs::canonicalize(&abs).unwrap_or(abs)
        })
        .collect()
}

/// Characterization tests: pin the enumeration-set semantics of
/// `staged_files` and `diff_base_files` so the genesis::git migration is
/// pass-to-pass. Fixtures use plain `git` subprocess commands only.
#[cfg(test)]
mod characterization_tests {
    use super::*;
    use tempfile::TempDir;

    fn run_git(root: &Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("spawn git");
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn commit(root: &Path, message: &str) {
        run_git(
            root,
            [
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.com",
                "commit",
                "-m",
                message,
            ]
            .as_slice(),
        );
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, body).unwrap();
    }

    /// Fresh repo fixture; returns the canonicalized root.
    fn repo() -> (TempDir, PathBuf) {
        let dir = TempDir::new().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        run_git(&root, &["init", "-q"]);
        (dir, root)
    }

    fn rel_paths(root: &Path, set: &HashSet<PathBuf>) -> Vec<String> {
        let mut names: Vec<String> = set
            .iter()
            .map(|p| {
                p.strip_prefix(root)
                    .unwrap_or(p)
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        names.sort();
        names
    }

    #[test]
    fn staged_files_is_staged_only() {
        let (_d, root) = repo();
        write(&root, "base.py", "def a(): pass\n");
        write(&root, "unstaged_mod.py", "def u(): pass\n");
        run_git(&root, &["add", "base.py", "unstaged_mod.py"]);
        commit(&root, "base");

        // Staged: new file + modification of a tracked file.
        write(&root, "staged_new.py", "def n(): pass\n");
        run_git(&root, &["add", "staged_new.py"]);
        write(&root, "base.py", "def a(): return 1\n");
        run_git(&root, &["add", "base.py"]);

        // Unstaged: committed file modified in the worktree, not staged.
        write(&root, "unstaged_mod.py", "def u(): return 2\n");

        // Untracked: never added.
        write(&root, "untracked.py", "def t(): pass\n");

        let staged = staged_files(&root).unwrap();
        assert_eq!(rel_paths(&root, &staged), vec!["base.py", "staged_new.py"]);
    }

    #[test]
    fn staged_files_excludes_staged_deletion() {
        let (_d, root) = repo();
        write(&root, "a.py", "def a(): pass\n");
        write(&root, "b.py", "def b(): pass\n");
        run_git(&root, &["add", "."]);
        commit(&root, "base");

        // Stage a deletion (file remains on disk, untracked).
        run_git(&root, &["rm", "--cached", "-q", "a.py"]);

        let staged = staged_files(&root).unwrap();
        assert!(
            staged.is_empty(),
            "expected empty, got {:?}",
            rel_paths(&root, &staged)
        );
    }

    #[test]
    fn staged_files_rename_reports_new_path_only() {
        let (_d, root) = repo();
        write(&root, "old.py", "def a(): pass\n");
        run_git(&root, &["add", "old.py"]);
        commit(&root, "base");

        run_git(&root, &["mv", "old.py", "new.py"]);

        let staged = staged_files(&root).unwrap();
        assert_eq!(rel_paths(&root, &staged), vec!["new.py"]);
    }

    #[test]
    fn staged_files_unborn_head_reports_staged_files() {
        let (_d, root) = repo();
        write(&root, "a.py", "def a(): pass\n");
        run_git(&root, &["add", "a.py"]);

        let staged = staged_files(&root).unwrap();
        assert_eq!(rel_paths(&root, &staged), vec!["a.py"]);
    }

    #[test]
    fn staged_files_outside_repo_errors() {
        let dir = TempDir::new().unwrap();
        let root = std::fs::canonicalize(dir.path()).unwrap();
        assert!(staged_files(&root).is_err());
    }

    #[test]
    fn diff_base_files_excludes_deleted_and_untracked() {
        let (_d, root) = repo();
        write(&root, "a.py", "def a(): pass\n");
        write(&root, "b.py", "def b(): pass\n");
        run_git(&root, &["add", "."]);
        commit(&root, "base");
        run_git(&root, &["tag", "base-tag"]);

        // Modify a.py, delete b.py, add c.py; leave d.py untracked.
        write(&root, "a.py", "def a(): return 1\n");
        run_git(&root, &["rm", "-q", "b.py"]);
        write(&root, "c.py", "def c(): pass\n");
        run_git(&root, &["add", "a.py", "c.py"]);
        commit(&root, "change");
        write(&root, "d.py", "def d(): pass\n");

        let files = diff_base_files(&root, "base-tag").unwrap();
        assert_eq!(rel_paths(&root, &files), vec!["a.py", "c.py"]);
    }

    #[test]
    fn diff_base_files_bad_base_errors() {
        let (_d, root) = repo();
        write(&root, "a.py", "def a(): pass\n");
        run_git(&root, &["add", "a.py"]);
        commit(&root, "base");

        assert!(diff_base_files(&root, "no-such-ref").is_err());
    }

    #[test]
    fn diff_base_files_unborn_head_errors() {
        let (_d, root) = repo();
        write(&root, "a.py", "def a(): pass\n");
        run_git(&root, &["add", "a.py"]);

        assert!(diff_base_files(&root, "HEAD").is_err());
    }
}
