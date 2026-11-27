use anyhow::{anyhow, Context, Result};
use genesis::git::{self, EnvPolicy, GitError};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Delta kinds kept from the libgit2 behavior this module replaced:
/// Added, Modified, Renamed, Copied, Typechange — deletions (D) and
/// unmerged entries (U) are excluded from every query.
const KEEP_DIFF_FILTER: &str = "--diff-filter=ACMRT";

/// Emit paths verbatim instead of C-style quoted; libgit2 never quoted
/// paths, so `core.quotePath` stays off to keep non-ASCII names intact.
const QUOTEPATH_OFF: [&str; 2] = ["-c", "core.quotePath=false"];

/// Returns canonical absolute paths of files currently staged in the git index.
/// Deleted files are excluded — only Added, Modified, Renamed, Copied, Typechange.
pub fn staged_files(cwd: &Path) -> Result<HashSet<PathBuf>> {
    let root = repo_root(cwd)?;
    staged_paths(&root)
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
    let root = repo_root(cwd)?;

    let mut files = HashSet::new();
    // `base_ref..HEAD`: commits between the reviewed base and HEAD. A 128
    // "unknown revision" here means `base_ref` did not resolve.
    let committed = match run_git(
        &root,
        &["diff", "--name-only", KEEP_DIFF_FILTER, base_ref, "HEAD"],
    ) {
        Ok(out) => out,
        Err(err) if is_unknown_revision(&err) => {
            return Err(anyhow!(
                "failed to resolve ref '{base_ref}' — is it fetched?"
            ));
        }
        Err(err) => return Err(err).context("failed to diff base against HEAD"),
    };
    files.extend(paths_from_stdout(&root, &committed.stdout));

    // `HEAD..index`: staged changes, including freshly `git add`ed files.
    files.extend(staged_paths(&root)?);

    // `index..worktree`: unstaged edits still pending before the commit.
    let pending = run_git(&root, &["diff", "--name-only", KEEP_DIFF_FILTER])
        .context("failed to diff worktree against index")?;
    files.extend(paths_from_stdout(&root, &pending.stdout));
    Ok(files)
}

/// Staged paths (HEAD..index) relative to `root`, deletions excluded.
///
/// On an unborn branch (empty repo) `diff --cached HEAD` fails, while plain
/// `diff --cached` diffs the index against the empty tree — the same answer
/// the libgit2 empty-tree comparison produced.
fn staged_paths(root: &Path) -> Result<HashSet<PathBuf>> {
    let out = match run_git(
        root,
        &["diff", "--cached", "--name-only", KEEP_DIFF_FILTER, "HEAD"],
    ) {
        Ok(out) => out,
        Err(err) if is_unknown_revision(&err) => {
            run_git(root, &["diff", "--cached", "--name-only", KEEP_DIFF_FILTER])
                .context("failed to diff index against HEAD")?
        }
        Err(err) => return Err(err).context("failed to diff index against HEAD"),
    };
    Ok(paths_from_stdout(root, &out.stdout))
}

/// Locate the working-tree root enclosing `cwd`.
fn repo_root(cwd: &Path) -> Result<PathBuf> {
    git::repo_root_from(cwd)
        .map_err(|err| anyhow!("failed to open git repository (is this a git repo?): {err}"))
}

/// Run git in `root` with hook-context env inherited.
fn run_git(root: &Path, args: &[&str]) -> Result<git::GitOutput> {
    let mut full: Vec<&str> = QUOTEPATH_OFF.to_vec();
    full.extend_from_slice(args);
    Ok(git::run(root, EnvPolicy::Inherit, &full)?)
}

/// Exit 128 complaining about an unknown or bad revision: an unborn branch
/// (empty repo) or an unresolvable ref (the same trigger libgit2 reported
/// as a failed revparse).
fn is_unknown_revision(err: &anyhow::Error) -> bool {
    err.downcast_ref::<GitError>().is_some_and(|git_err| {
        matches!(git_err, GitError::Git { code: 128, stderr, .. }
            if stderr.contains("unknown revision") || stderr.contains("bad revision"))
    })
}

/// One canonical absolute path per output line.
fn paths_from_stdout(root: &Path, stdout: &[u8]) -> HashSet<PathBuf> {
    String::from_utf8_lossy(stdout)
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let abs = root.join(line);
            std::fs::canonicalize(&abs).unwrap_or(abs)
        })
        .collect()
}
