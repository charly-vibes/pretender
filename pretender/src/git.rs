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

/// Returns canonical absolute paths of files changed relative to `base_ref`,
/// covering the full checked-out state rather than only committed history:
/// `base_ref..HEAD`, staged changes (HEAD..index), and unstaged worktree
/// edits (index..worktree).
///
/// At pre-commit time the pending changes are staged or still in the worktree,
/// so a base..HEAD-only diff would evaluate an earlier commit's files while
/// the edit under review stays invisible. An untracked file that has just been
/// `git add`ed is an index addition and is included via HEAD..index.
pub fn diff_base_files(cwd: &Path, base_ref: &str) -> Result<HashSet<PathBuf>> {
    let repo = git2::Repository::discover(cwd)
        .context("failed to open git repository (is this a git repo?)")?;
    let root = workdir(&repo)?;
    let index = repo.index().context("failed to read git index")?;
    let diffs = base_diffs(&repo, base_ref, &index)?;

    let mut files = paths_from_diff(&root, &diffs.committed);
    files.extend(paths_from_diff(&root, &diffs.staged));
    files.extend(paths_from_diff(&root, &diffs.pending));
    Ok(files)
}

/// Diffs covering the full checked-out state relative to `base_ref`.
struct BaseDiffs<'r> {
    committed: git2::Diff<'r>,
    staged: git2::Diff<'r>,
    pending: git2::Diff<'r>,
}

/// Build the three diffs: `base_ref..HEAD` (committed), `HEAD..index`
/// (staged), and `index..worktree` (pending).
fn base_diffs<'r>(
    repo: &'r git2::Repository,
    base_ref: &str,
    index: &git2::Index,
) -> Result<BaseDiffs<'r>> {
    let base_tree = resolve_tree(repo, base_ref)?;
    let head_tree = head_tree(repo);
    Ok(BaseDiffs {
        committed: committed_diff(repo, &base_tree, head_tree.as_ref())?,
        staged: staged_diff(repo, head_tree.as_ref(), index)?,
        pending: pending_diff(repo, index)?,
    })
}

/// `base_ref..HEAD`: commits between the reviewed base and HEAD.
fn committed_diff<'r>(
    repo: &'r git2::Repository,
    base_tree: &git2::Tree<'r>,
    head_tree: Option<&git2::Tree<'r>>,
) -> Result<git2::Diff<'r>> {
    repo.diff_tree_to_tree(Some(base_tree), head_tree, None)
        .context("failed to diff base against HEAD")
}

/// `HEAD..index`: staged changes, including freshly `git add`ed files.
fn staged_diff<'r>(
    repo: &'r git2::Repository,
    head_tree: Option<&git2::Tree<'r>>,
    index: &git2::Index,
) -> Result<git2::Diff<'r>> {
    repo.diff_tree_to_index(head_tree, Some(index), None)
        .context("failed to diff index against HEAD")
}

/// `index..worktree`: unstaged edits still pending before the commit.
fn pending_diff<'r>(repo: &'r git2::Repository, index: &git2::Index) -> Result<git2::Diff<'r>> {
    repo.diff_index_to_workdir(Some(index), None)
        .context("failed to diff worktree against index")
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
