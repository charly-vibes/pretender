//! Shared helpers for the governance gate tests (pretender-5zo.19).
//!
//! Included by each gate test file via `#[path = "governance/util.rs"] mod
//! util;` — `tests/` subdirectories are not auto-discovered as test targets.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Repository root, or `None` when running outside the repository
/// (packaged crate, e.g. `cargo publish` tarball).
pub fn repo_root() -> Option<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.parent()?;
    let wired = root.join("justfile").is_file();
    wired.then(|| root.to_path_buf())
}

pub fn read(root: &Path, relative: &str) -> String {
    let path = root.join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// A neutral global git config so the hermetic gate run ignores the
/// developer's user-level git settings.
fn neutral_git_config() -> PathBuf {
    let path = std::env::temp_dir().join("pretender-governance-neutral-gitconfig");
    std::fs::write(&path, b"").expect("write neutral git config");
    path
}

/// Run the freshly-built `pretender check . --format json` in gate mode from
/// the repo root (pretender.toml selects gate mode; pretender-baseline.json
/// sits at the root), hermetic w.r.t. the developer's git config.
/// Returns (exit success, stdout).
pub fn hermetic_check(root: &Path) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_pretender"))
        .args(["check", ".", "--format", "json"])
        .current_dir(root)
        .env("GIT_CONFIG_GLOBAL", neutral_git_config())
        .output()
        .expect("run pretender check");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    (output.status.success(), stdout)
}
