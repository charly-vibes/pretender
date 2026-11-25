//! JUnit testcase → candidate source file resolution.

use std::path::{Path, PathBuf};

/// Common language extensions tried for classname-derived candidates.
const EXTENSIONS: &[&str] = &[
    "py", "rs", "js", "ts", "jsx", "tsx", "go", "java", "rb", "c", "cpp", "h", "hpp",
];

/// Resolve a JUnit testcase to a candidate source file path.
///
/// Resolution order:
/// 1. If `file` is present, use it directly.
/// 2. Otherwise, derive a path from `classname`: replace `.` with `/`,
///    append the detected language extension, and search under
///    `classname_root` (default `tests`). The first existing file wins.
/// 3. If unresolved, returns `None` (base `test` role with a note).
pub fn resolve_testcase_file(
    file: Option<&Path>,
    classname: &str,
    classname_root: &str,
    search_root: &Path,
) -> Option<PathBuf> {
    resolve_from_file_attr(file, search_root)
        .or_else(|| resolve_from_classname(classname, classname_root, search_root))
}

/// Step 1: use the `file` attribute directly, else relative to the search root.
fn resolve_from_file_attr(file: Option<&Path>, search_root: &Path) -> Option<PathBuf> {
    let f = file?;
    if f.exists() {
        return Some(f.to_path_buf());
    }
    let rel = search_root.join(f);
    rel.exists().then_some(rel)
}

/// Steps 2–3: derive a path from `classname` under `classname_root` and
/// return the first extension candidate that exists on disk.
fn resolve_from_classname(
    classname: &str,
    classname_root: &str,
    search_root: &Path,
) -> Option<PathBuf> {
    if classname.is_empty() {
        return None;
    }
    let classname_path = classname.replace('.', "/");
    let root = if classname_root.is_empty() {
        PathBuf::from("tests")
    } else {
        PathBuf::from(classname_root)
    };
    EXTENSIONS
        .iter()
        .map(|ext| root.join(format!("{classname_path}.{ext}")))
        .find(|candidate| search_root.join(candidate).exists())
}
