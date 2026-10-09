use anyhow::{anyhow, Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Returns canonical absolute paths of files currently staged in the git index.
/// Deleted files are excluded — only Added, Modified, Renamed, Copied, Typechange.
pub fn staged_files(cwd: &Path) -> Result<HashSet<PathBuf>> {
    let repo = git2::Repository::discover(cwd)
        .context("failed to open git repository (is this a git repo?)")?;
    let root = workdir(&repo)?;
    let index = repo.index().context("failed to read git index")?;
    let head_tree = head_tree(&repo);
    let diff = repo
        .diff_tree_to_index(head_tree.as_ref(), Some(&index), None)
        .context("failed to diff index against HEAD")?;
    Ok(paths_from_diff(&root, &diff))
}

/// Returns canonical absolute paths of files changed between `base_ref` and HEAD.
pub fn diff_base_files(cwd: &Path, base_ref: &str) -> Result<HashSet<PathBuf>> {
    let repo = git2::Repository::discover(cwd)
        .context("failed to open git repository (is this a git repo?)")?;
    let root = workdir(&repo)?;
    let base_tree = resolve_tree(&repo, base_ref)?;
    let head_tree = repo
        .head()
        .context("failed to get HEAD")?
        .peel_to_commit()
        .context("failed to peel HEAD to commit")?
        .tree()
        .context("failed to get HEAD commit tree")?;
    let diff = repo
        .diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)
        .context("failed to diff trees")?;
    Ok(paths_from_diff(&root, &diff))
}

fn workdir(repo: &git2::Repository) -> Result<PathBuf> {
    repo.workdir()
        .ok_or_else(|| anyhow!("git repository has no working directory (bare repo?)"))
        .map(Path::to_path_buf)
}

// Returns None for an empty repo (no HEAD yet); diff against empty tree is correct.
fn head_tree(repo: &git2::Repository) -> Option<git2::Tree<'_>> {
    repo.head().ok()?.peel_to_commit().ok()?.tree().ok()
}

fn resolve_tree<'r>(repo: &'r git2::Repository, refname: &str) -> Result<git2::Tree<'r>> {
    repo.revparse_single(refname)
        .with_context(|| format!("failed to resolve ref '{refname}' — is it fetched?"))?
        .peel_to_commit()
        .with_context(|| format!("failed to peel '{refname}' to a commit"))?
        .tree()
        .context("failed to get commit tree")
}

fn paths_from_diff(root: &Path, diff: &git2::Diff<'_>) -> HashSet<PathBuf> {
    diff.deltas()
        .filter_map(|delta| {
            use git2::Delta;
            match delta.status() {
                Delta::Added
                | Delta::Modified
                | Delta::Renamed
                | Delta::Copied
                | Delta::Typechange => delta.new_file().path().map(|p| {
                    let abs = root.join(p);
                    std::fs::canonicalize(&abs).unwrap_or(abs)
                }),
                _ => None,
            }
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
        assert!(staged.is_empty(), "expected empty, got {:?}", rel_paths(&root, &staged));
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
