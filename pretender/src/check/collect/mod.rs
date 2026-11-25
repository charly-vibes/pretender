//! Input file collection for the check pipeline.
//!
//! Walks the requested paths, honors `.gitignore` by default via the
//! `ignore` walker, applies configured exclude globs, and keeps only
//! source files with a parser-backed extension.
//!
//! # Responsibilities
//! `collect_input_files` and its override variant are the single entry
//! points; `collect_path` recurses per path with the walker and manual
//! fallback; the exclude-glob helpers normalize the two `./`-spelling
//! patterns users write. Walk and exclude behavior tests live in
//! `super::collect::tests`.
//!
//! # Rationale
//! Extracted verbatim from main.rs (pretender-5zo.2.2); the original
//! 67-line `collect_path` and 37-line `is_supported_source` split into
//! verbatim-behaving helpers so walk order and error semantics stay
//! identical within the structural limits. Inline tests moved to
//! `tests.rs` (pretender-5zo.15.12).
use anyhow::{Context, Result};
use pretender::config::Config;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

mod exclude;

use exclude::{build_exclude_set, is_excluded};

/// Collect input files with the default (gitignore-respecting) walk.
pub(crate) fn collect_input_files(paths: &[PathBuf], config: &Config) -> Result<Vec<PathBuf>> {
    collect_input_files_with_override(paths, config, false)
}

/// Restrict walked files to the staged/diff-allowed set, canonicalizing
/// relative paths against `cwd`.
pub(super) fn apply_file_filter(
    files: Vec<PathBuf>,
    allowed: Option<&HashSet<PathBuf>>,
    cwd: &Path,
) -> Vec<PathBuf> {
    let Some(allowed) = allowed else {
        return files;
    };
    files
        .into_iter()
        .filter(|f| {
            let joined = cwd.join(f);
            let canonical = std::fs::canonicalize(&joined).unwrap_or(joined);
            allowed.contains(&canonical)
        })
        .collect()
}

/// Walk `paths`, optionally restricted to `scan_ignored`-aware behavior and
/// the gitignore defaults.
///
/// pretender-094: honor .gitignore by default so generated/vendored/scratch
/// dirs stop being scanned without per-repo exclude lists. `--scan-ignored`
/// (CLI) restores the exhaustive walk regardless of config.
pub(super) fn collect_input_files_with_override(
    paths: &[PathBuf],
    config: &Config,
    scan_ignored: bool,
) -> Result<Vec<PathBuf>> {
    let exclude_set = build_exclude_set(&config.pretender.exclude)?;
    let mut files = Vec::new();
    let respect_gitignore = config.pretender.respect_gitignore && !scan_ignored;
    let opts = CollectOpts {
        exclude: &exclude_set,
        respect_gitignore,
    };
    for path in paths {
        collect_path(&opts, path, false, &mut files)?;
    }
    files.sort();
    files.dedup();
    Ok(files)
}

/// True if `path` has an extension mapped to a parser in `get_parser`.
fn is_supported_source(path: &Path) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .is_some_and(is_supported_extension)
}

/// Parser-backed extensions, derived from the declarative registry — the
/// single source of truth shared with scan/parsers.rs dispatch.
fn is_supported_extension(ext: &str) -> bool {
    pretender::registry::supports_extension(ext)
}

/// Walk configuration shared by `collect_path` and its recursion.
struct CollectOpts<'a> {
    exclude: &'a globset::GlobSet,
    respect_gitignore: bool,
}

/// Collect files under one path, recursing into directories.
///
/// Files discovered by walking a directory must have a supported source
/// extension; non-source files (.gitignore, README, etc.) are skipped
/// silently. Explicitly passed paths keep the loud `get_parser` error so
/// typos surface.
fn collect_path(
    opts: &CollectOpts<'_>,
    path: &Path,
    walked: bool,
    out: &mut Vec<PathBuf>,
) -> Result<()> {
    if is_excluded(opts.exclude, path) {
        return Ok(());
    }
    if path.is_file() {
        return collect_file(path, walked, out);
    }
    if path.is_dir() {
        return walk_dir(opts, path, out);
    }
    // Not a regular file or directory — skip silently.
    Ok(())
}

/// A directly passed or walked regular file. Walked files must have a
/// supported source extension; explicit paths bypass that filter so
/// intentionally passed files are always collected.
fn collect_file(path: &Path, walked: bool, out: &mut Vec<PathBuf>) -> Result<()> {
    if walked && !is_supported_source(path) {
        return Ok(());
    }
    out.push(path.to_path_buf());
    Ok(())
}

/// Dispatch a directory to the gitignore-aware or manual walk.
fn walk_dir(opts: &CollectOpts<'_>, path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if opts.respect_gitignore {
        walk_with_gitignore(opts, path, out)
    } else {
        walk_read_dir(opts, path, out)
    }
}

/// Directory walk via the `ignore` walker.
///
/// pretender-094: delegate to the `ignore` walker so nested .gitignore
/// files, the global/core.excludesFile, and .git/info/exclude are all
/// honored. Outside a git repo (require_git, the default) the walk is
/// unfiltered — same as the manual fallback below. Hidden entries are
/// walked (pre-existing behavior); `.git` itself is pruned explicitly.
fn walk_with_gitignore(opts: &CollectOpts<'_>, path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    let mut builder = ignore::WalkBuilder::new(path);
    builder
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .hidden(false);
    builder.filter_entry(|e| e.file_name() != ".git");
    for entry in builder.build() {
        let entry =
            entry.with_context(|| format!("failed to walk directory: {}", path.display()))?;
        walk_entry(opts, entry.path(), path, out);
    }
    Ok(())
}

/// One walked entry: skip the walk root, excluded paths, and unsupported
/// files; collect the rest.
fn walk_entry(opts: &CollectOpts<'_>, entry_path: &Path, root: &Path, out: &mut Vec<PathBuf>) {
    if entry_path == root || is_excluded(opts.exclude, entry_path) {
        return;
    }
    if entry_path.is_file() && is_supported_source(entry_path) {
        out.push(entry_path.to_path_buf());
    }
}

/// Manual directory walk used when gitignore handling is disabled.
fn walk_read_dir(opts: &CollectOpts<'_>, path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(path)
        .with_context(|| format!("failed to read directory: {}", path.display()))?
    {
        let entry = entry?;
        let entry_path = entry.path();
        // Skip non-regular, non-directory entries (sockets, FIFOs, etc.)
        // rather than erroring, since these can appear in .git/ and other
        // infrastructure directories.
        if entry_path.is_file() || entry_path.is_dir() {
            collect_path(opts, &entry_path, true, out)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
