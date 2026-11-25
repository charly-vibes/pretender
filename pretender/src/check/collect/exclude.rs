//! Exclude-pattern compilation and path matching.
//!
//! # Responsibilities
//! Compile configured exclude globs, normalize walker-relative patterns,
//! strip leading `.` path components, and test paths against the exclude
//! set. Everything the file-collection walk needs to decide "is this path
//! excluded".
//!
//! # Rationale
//! Split from the collection walk (pretender-5zo.15.12) along the natural
//! exclude-pattern seam so both halves stay within the structural limits.
//! Loop state (`let mut` strip/builder accumulators) was restructured into
//! `successors`/`fold` pipelines with identical results, in the same
//! order, keeping the module's mutability ratio low.
use std::path::Path;

use anyhow::{Context, Result};

/// Compile the configured exclude patterns into a glob set.
///
/// Patterns are normalized to walker-relative form (`./tests/**` becomes
/// `tests/**`) so a single set matches both spellings users write.
pub(super) fn build_exclude_set(patterns: &[String]) -> Result<globset::GlobSet> {
    let globs = patterns
        .iter()
        .map(|pattern| {
            globset::Glob::new(normalize_exclude_pattern(pattern))
                .with_context(|| format!("invalid exclude pattern: {}", pattern))
        })
        .collect::<Result<Vec<_>>>()?;
    let builder = globs
        .into_iter()
        .fold(globset::GlobSetBuilder::new(), |mut builder, glob| {
            builder.add(glob);
            builder
        });
    builder.build().context("failed to build exclude GlobSet")
}

/// Strip any leading `./` components so `./tests/**` and `tests/**` compile
/// to the same glob.
fn normalize_exclude_pattern(pattern: &str) -> &str {
    // The strip chain's last link is the pattern with every leading `./`
    // removed — identical to the original strip loop.
    std::iter::successors(Some(pattern), |rest| rest.strip_prefix("./"))
        .last()
        .unwrap_or(pattern)
}

/// `path` without its leading `.` component, when it has one.
///
/// Walking `.` (the CLI default) yields entries like `./tests/x.rs`, and
/// explicit `./dir` arguments carry the same prefix; neither matches
/// walker-relative globs such as `tests/**` until stripped.
pub(super) fn strip_leading_cur_dir(path: &Path) -> Option<&Path> {
    // Walk the leading-`.` strip chain and keep its last link; the chain
    // index is the strip count. A link that strips to the empty path ends
    // the chain and keeps the previous path — the original loop's break,
    // so stripping `.` alone still yields `Some` with the path unchanged.
    let (current, stripped) = std::iter::successors(Some(path), |p| p.strip_prefix(".").ok())
        .enumerate()
        .last()
        .map_or((path, false), |(index, stripped)| {
            let current = if stripped.as_os_str().is_empty() {
                path
            } else {
                stripped
            };
            (current, index > 0)
        });
    stripped.then_some(current)
}

/// True when an exclude glob matches `path`, testing both the raw path and
/// its walker-relative form.
pub(super) fn is_excluded(exclude_set: &globset::GlobSet, path: &Path) -> bool {
    exclude_set.is_match(path)
        || strip_leading_cur_dir(path).is_some_and(|rest| exclude_set.is_match(rest))
}
