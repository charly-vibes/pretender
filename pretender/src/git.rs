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
