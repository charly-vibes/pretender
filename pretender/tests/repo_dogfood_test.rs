//! Repo-level governance tests: pretender's structural gate must be wired into
//! the local gate (`just ci`) and into CI, not just into a developer's
//! lefthook hook.
//!
//! These tests read repository files, so they no-op when the crate is packaged
//! without the surrounding repository (e.g. `cargo publish` tarball).

use std::path::{Path, PathBuf};

/// Repository root, or `None` when running outside the repository (packaged crate).
fn repo_root() -> Option<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest_dir.parent()?;
    if root.join("justfile").is_file() {
        Some(root.to_path_buf())
    } else {
        None
    }
}

fn read(root: &Path, relative: &str) -> String {
    let path = root.join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Each row is (file to read, substring that must be present, why it matters).
const WIRING: &[(&str, &str, &str)] = &[
    (
        "justfile",
        "\ndogfood:",
        "justfile must define a `dogfood` recipe that runs pretender against this repo",
    ),
    (
        "justfile",
        "check .",
        "the `dogfood` recipe must invoke pretender's check on the repo root",
    ),
    (
        ".github/workflows/ci.yml",
        "just ci",
        "CI must run `just ci`, which now includes the dogfood gate",
    ),
    (
        "pretender.toml",
        "mode = \"gate\"",
        "pretender.toml must select gate mode so the bare `pretender check .` gates",
    ),
    (
        "pretender.toml",
        "fixtures",
        "pretender.toml must exclude deliberate test fixtures from the gate scope",
    ),
];

#[test]
fn repo_files_wire_the_dogfood_gate() {
    let Some(root) = repo_root() else { return };
    for (relative, needle, rationale) in WIRING {
        let contents = read(&root, relative);
        assert!(
            contents.contains(needle),
            "{rationale} — {relative} must contain {needle:?}"
        );
    }
}

#[test]
fn just_ci_runs_the_dogfood_gate() {
    let Some(root) = repo_root() else { return };
    let justfile = read(&root, "justfile");
    let ci_line = justfile
        .lines()
        .find(|line| line.trim_start().starts_with("ci:"))
        .expect("justfile must define a `ci` recipe");
    assert!(
        ci_line.contains("dogfood"),
        "`just ci` must run the dogfood gate, got: {ci_line}"
    );
}

#[test]
fn gate_baseline_is_committed() {
    let Some(root) = repo_root() else { return };
    assert!(
        root.join("pretender-baseline.json").is_file(),
        "pretender-baseline.json must be committed so known findings stay green"
    );
}
