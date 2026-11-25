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
//! Repo/file seeding and the check subprocess run live in `write` and
//! `run_check`/`stdout_of`; every behavioral assertion stays in the tests.

use crate::common::*;
use std::path::Path;

/// Writes `path` (creating parent dirs), panicking with the path in the
/// message on failure.
fn write(path: &Path, content: &str) -> std::path::PathBuf {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|e| panic!("mkdir {}: {e:?}", path.display()));
    }
    std::fs::write(path, content).unwrap_or_else(|e| panic!("write {}: {e:?}", path.display()));
    path.to_path_buf()
}

/// Drains a spawned command's stdout after asserting it succeeded.
fn stdout_of(cmd: &mut std::process::Command) -> String {
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to execute process: {e:?}"));
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Runs `pretender check` with `args` from `dir`, asserting success; returns stdout.
fn run_check(dir: &Path, args: &[&str]) -> String {
    stdout_of(
        pretender_cmd()
            .args(args)
            .current_dir(dir)
            .env("NO_COLOR", "1"),
    )
}

#[test]
fn test_check_staged_only_scans_staged_files() {
    let dir = tempdir();
    git_init(&dir);

    // Commit a baseline file so HEAD exists
    let baseline = write(&dir.join("baseline.py"), "def baseline(): pass\n");
    git_add(&dir, &baseline);
    git_commit(&dir, "baseline");

    // Stage file_a.py
    let file_a = write(&dir.join("file_a.py"), "def func_a(): pass\n");
    git_add(&dir, &file_a);

    // Write file_b.py but do NOT stage it
    write(&dir.join("file_b.py"), "def func_b(): pass\n");

    let stdout = run_check(&dir, &["check", ".", "--staged"]);
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
    let f = write(&dir.join("committed.py"), "def f(): pass\n");
    git_add(&dir, &f);
    git_commit(&dir, "init");

    // Nothing staged — check must succeed and list no files
    let stdout = run_check(&dir, &["check", ".", "--staged"]);
    assert!(
        !stdout.contains(".py"),
        "empty staging area must list no files: {stdout}"
    );
}

#[test]
fn test_check_staged_first_commit_no_head() {
    let dir = tempdir();
    git_init(&dir);

    // No prior commits — stage one file
    let f = write(&dir.join("first.py"), "def first(): pass\n");
    git_add(&dir, &f);

    let stdout = run_check(&dir, &["check", ".", "--staged"]);
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
    let file_old = write(&dir.join("file_old.py"), "def old(): pass\n");
    git_add(&dir, &file_old);
    git_commit(&dir, "first commit");

    // Second commit (HEAD): add file_new.py
    let file_new = write(&dir.join("file_new.py"), "def new_func(): pass\n");
    git_add(&dir, &file_new);
    git_commit(&dir, "second commit");

    // --diff-only --diff-base=HEAD~1 should only show file_new.py
    let stdout = run_check(&dir, &["check", ".", "--diff-only", "--diff-base=HEAD~1"]);
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

    let baseline = write(&dir.join("baseline.py"), "def baseline(): pass\n");
    git_add(&dir, &baseline);
    let unchanged = write(&dir.join("unchanged.py"), "def unchanged(): pass\n");
    git_add(&dir, &unchanged);
    git_commit(&dir, "baseline commit");

    // Pending worktree edit: base..HEAD has nothing, so a commit-only diff
    // would evaluate an older commit and miss this file entirely.
    write(&dir.join("baseline.py"), "def baseline():\n    return 1\n");

    let stdout = run_check(&dir, &["check", ".", "--diff-only", "--diff-base=HEAD"]);
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

    let baseline = write(&dir.join("baseline.py"), "def baseline(): pass\n");
    git_add(&dir, &baseline);
    git_commit(&dir, "baseline commit");

    // Brand-new file, staged but never committed: an index addition.
    let fresh = write(&dir.join("fresh.py"), "def fresh(): pass\n");
    git_add(&dir, &fresh);

    let stdout = run_check(&dir, &["check", ".", "--diff-only", "--diff-base=HEAD"]);
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
    write(
        &dir.join("pretender.toml"),
        "[pretender]\nexclude = [\"tests/**\", \"vendor/**\"]\n",
    );

    for (relative, source) in [
        ("src/app.py", "def app(): pass\n"),
        ("tests/test_a.py", "def test_a(): pass\n"),
        ("vendor/lib.py", "def vendored(): pass\n"),
    ] {
        write(&dir.join(relative), source);
    }

    let stdout = stdout_of(&mut check_in(&dir, Path::new(".")));
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
    write(
        &dir.join("pretender.toml"),
        "[pretender]\nexclude = [\"./tests/**\"]\n",
    );
    write(&dir.join("tests/test_a.py"), "def test_a(): pass\n");
    write(&dir.join("app.py"), "def app(): pass\n");

    let dot_stdout = stdout_of(&mut check_in(&dir, Path::new(".")));
    assert!(dot_stdout.contains("app.py"), "stdout: {dot_stdout}");
    assert!(
        !dot_stdout.contains("test_a.py"),
        "./tests/** must exclude the dot-prefixed walk: {dot_stdout}"
    );

    // Walking `tests` directly yields paths without the `./` prefix, so the
    // pattern only matches once its own leading `./` is normalized away.
    let bare_stdout = stdout_of(&mut check_in(&dir, Path::new("tests")));
    assert!(
        !bare_stdout.contains("test_a.py"),
        "./tests/** must exclude walked tests/test_a.py: {bare_stdout}"
    );
}
