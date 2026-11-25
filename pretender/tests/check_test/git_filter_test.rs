//! `check` git-aware path filtering tests.
//!
//! # Responsibilities
//! Pin `--staged` and `--diff-only` selection semantics (staged-only,
//! unstaged worktree edits, newly staged untracked files, empty staging
//! area, first commit without HEAD) and exclude-glob matching from a dot
//! working directory.
//!
//! # Rationale
//! Moved verbatim from `tests/cli_test.rs` (wave 3 per-command split);
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms.

use crate::common::*;
use std::path::Path;

#[test]
fn test_check_staged_only_scans_staged_files() {
    let dir = tempdir();
    git_init(&dir);

    // Commit a baseline file so HEAD exists
    let baseline = dir.join("baseline.py");
    std::fs::write(&baseline, "def baseline(): pass\n")
        .unwrap_or_else(|e| panic!("write baseline: {e:?}"));
    git_add(&dir, &baseline);
    git_commit(&dir, "baseline");

    // Stage file_a.py
    let file_a = dir.join("file_a.py");
    std::fs::write(&file_a, "def func_a(): pass\n")
        .unwrap_or_else(|e| panic!("write file_a: {e:?}"));
    git_add(&dir, &file_a);

    // Write file_b.py but do NOT stage it
    let file_b = dir.join("file_b.py");
    std::fs::write(&file_b, "def func_b(): pass\n")
        .unwrap_or_else(|e| panic!("write file_b: {e:?}"));

    let output = pretender_cmd()
        .args(["check", ".", "--staged"])
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("file_a.py"),
        "expected file_a.py in stdout: {stdout}"
    );
    assert!(
        !stdout.contains("file_b.py"),
        "unexpected file_b.py in stdout (not staged): {stdout}"
    );
}

#[test]
fn test_check_staged_empty_staging_area_exits_success() {
    let dir = tempdir();
    git_init(&dir);

    // Commit a file so HEAD exists
    let f = dir.join("committed.py");
    std::fs::write(&f, "def f(): pass\n").unwrap_or_else(|e| panic!("write f: {e:?}"));
    git_add(&dir, &f);
    git_commit(&dir, "init");

    // Nothing staged — check should succeed with no file output
    let output = pretender_cmd()
        .args(["check", ".", "--staged"])
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_check_staged_first_commit_no_head() {
    let dir = tempdir();
    git_init(&dir);

    // No prior commits — stage one file
    let f = dir.join("first.py");
    std::fs::write(&f, "def first(): pass\n").unwrap_or_else(|e| panic!("write f: {e:?}"));
    git_add(&dir, &f);

    let output = pretender_cmd()
        .args(["check", ".", "--staged"])
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("first.py"),
        "expected first.py in stdout: {stdout}"
    );
}

#[test]
fn test_check_diff_only_filters_to_changed_files() {
    let dir = tempdir();
    git_init(&dir);

    // First commit: file_old.py
    let file_old = dir.join("file_old.py");
    std::fs::write(&file_old, "def old(): pass\n")
        .unwrap_or_else(|e| panic!("write file_old: {e:?}"));
    git_add(&dir, &file_old);
    git_commit(&dir, "first commit");

    // Second commit (HEAD): add file_new.py
    let file_new = dir.join("file_new.py");
    std::fs::write(&file_new, "def new_func(): pass\n")
        .unwrap_or_else(|e| panic!("write file_new: {e:?}"));
    git_add(&dir, &file_new);
    git_commit(&dir, "second commit");

    // --diff-only --diff-base=HEAD~1 should only show file_new.py
    let output = pretender_cmd()
        .args(["check", ".", "--diff-only", "--diff-base=HEAD~1"])
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("file_new.py"),
        "expected file_new.py in stdout: {stdout}"
    );
    assert!(
        !stdout.contains("file_old.py"),
        "unexpected file_old.py in stdout (unchanged): {stdout}"
    );
}

#[test]
fn test_check_diff_only_includes_unstaged_worktree_edits() {
    let dir = tempdir();
    git_init(&dir);

    let baseline = dir.join("baseline.py");
    std::fs::write(&baseline, "def baseline(): pass\n")
        .unwrap_or_else(|e| panic!("write baseline: {e:?}"));
    git_add(&dir, &baseline);
    let unchanged = dir.join("unchanged.py");
    std::fs::write(&unchanged, "def unchanged(): pass\n")
        .unwrap_or_else(|e| panic!("write unchanged: {e:?}"));
    git_add(&dir, &unchanged);
    git_commit(&dir, "baseline commit");

    // Pending worktree edit: base..HEAD has nothing, so a commit-only diff
    // would evaluate an older commit and miss this file entirely.
    std::fs::write(&baseline, "def baseline():\n    return 1\n")
        .unwrap_or_else(|e| panic!("write baseline: {e:?}"));

    let output = pretender_cmd()
        .args(["check", ".", "--diff-only", "--diff-base=HEAD"])
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("baseline.py"),
        "expected pending edit baseline.py in stdout: {stdout}"
    );
    assert!(
        !stdout.contains("unchanged.py"),
        "unexpected unchanged.py in stdout: {stdout}"
    );
}

#[test]
fn test_check_diff_only_includes_newly_staged_untracked_file() {
    let dir = tempdir();
    git_init(&dir);

    let baseline = dir.join("baseline.py");
    std::fs::write(&baseline, "def baseline(): pass\n")
        .unwrap_or_else(|e| panic!("write baseline: {e:?}"));
    git_add(&dir, &baseline);
    git_commit(&dir, "baseline commit");

    // Brand-new file, staged but never committed: an index addition.
    let fresh = dir.join("fresh.py");
    std::fs::write(&fresh, "def fresh(): pass\n").unwrap_or_else(|e| panic!("write fresh: {e:?}"));
    git_add(&dir, &fresh);

    let output = pretender_cmd()
        .args(["check", ".", "--diff-only", "--diff-base=HEAD"])
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("fresh.py"),
        "expected newly staged fresh.py in stdout: {stdout}"
    );
    assert!(
        !stdout.contains("baseline.py"),
        "unexpected unchanged baseline.py in stdout: {stdout}"
    );
}

#[test]
fn test_check_exclude_globs_match_walked_paths_from_dot() {
    let dir = tempdir();
    std::fs::write(
        dir.join("pretender.toml"),
        "[pretender]\nexclude = [\"tests/**\", \"vendor/**\"]\n",
    )
    .unwrap_or_else(|e| panic!("write config: {e:?}"));

    for (relative, source) in [
        ("src/app.py", "def app(): pass\n"),
        ("tests/test_a.py", "def test_a(): pass\n"),
        ("vendor/lib.py", "def vendored(): pass\n"),
    ] {
        let full = dir.join(relative);
        std::fs::create_dir_all(full.parent().unwrap_or_else(|| panic!("parent dir")))
            .unwrap_or_else(|e| panic!("mkdir: {e:?}"));
        std::fs::write(full, source).unwrap_or_else(|e| panic!("write source: {e:?}"));
    }

    let output = check_in(&dir, Path::new("."))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));

    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("app.py"),
        "expected app.py in stdout: {stdout}"
    );
    assert!(
        !stdout.contains("test_a.py"),
        "tests/** must exclude the dot-prefixed walked path: {stdout}"
    );
    assert!(
        !stdout.contains("lib.py"),
        "vendor/** must exclude the dot-prefixed walked path: {stdout}"
    );
}

#[test]
fn test_check_exclude_dot_prefixed_glob_matches_walked_path() {
    let dir = tempdir();
    std::fs::write(
        dir.join("pretender.toml"),
        "[pretender]\nexclude = [\"./tests/**\"]\n",
    )
    .unwrap_or_else(|e| panic!("write config: {e:?}"));
    std::fs::create_dir_all(dir.join("tests")).unwrap_or_else(|e| panic!("mkdir tests: {e:?}"));
    std::fs::write(dir.join("tests").join("test_a.py"), "def test_a(): pass\n")
        .unwrap_or_else(|e| panic!("write test file: {e:?}"));
    std::fs::write(dir.join("app.py"), "def app(): pass\n")
        .unwrap_or_else(|e| panic!("write app file: {e:?}"));

    let dot = check_in(&dir, Path::new("."))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));
    assert!(
        dot.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&dot.stderr)
    );
    let dot_stdout = String::from_utf8_lossy(&dot.stdout);
    assert!(dot_stdout.contains("app.py"), "stdout: {dot_stdout}");
    assert!(
        !dot_stdout.contains("test_a.py"),
        "./tests/** must exclude the dot-prefixed walk: {dot_stdout}"
    );

    // Walking `tests` directly yields paths without the `./` prefix, so the
    // pattern only matches once its own leading `./` is normalized away.
    let bare = check_in(&dir, Path::new("tests"))
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));
    assert!(
        bare.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&bare.stderr)
    );
    let bare_stdout = String::from_utf8_lossy(&bare.stdout);
    assert!(
        !bare_stdout.contains("test_a.py"),
        "./tests/** must exclude walked tests/test_a.py: {bare_stdout}"
    );
}
