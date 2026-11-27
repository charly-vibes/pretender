//! `hooks` subcommand tests: managed-hook lifecycle and gate integration.
//!
//! # Responsibilities
//! Pin hooks install/uninstall — managed-shim creation and removal,
//! per-hook-name targeting (pre-push), hooks-dir creation when absent,
//! idempotent reinstall over Pretender-owned hooks, refusal on foreign
//! hooks, repo-root placement when run from a subdirectory, clean failure
//! outside a git repo, and the meter gate hook script stopping a violating
//! commit.
//!
//! # Rationale
//! Moved verbatim from `tests/cli_test.rs` (wave 3 per-command split);
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms.

use crate::common::*;

use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_meter_gate_hook_stops_commit() {
    // Ticket 15w acceptance, executable: the meter seeds a scratch repo with
    // a violating function, installs the hook, and asserts the gate stops the
    // commit with the function name + rule in the output.
    let meter = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts/meter-gate-hook.sh");
    let output = Command::new("sh")
        .arg(meter)
        .env("PRETENDER_BIN", pretender_bin())
        .env_remove("GIT_CONFIG_GLOBAL")
        .output()
        .unwrap_or_else(|e| panic!("run meter script: {e:?}"));
    assert!(
        output.status.success(),
        "meter failed; stdout+stderr: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_hooks_install_and_uninstall_manage_pretender_shim() {
    let dir = tempdir();
    git_init(&dir);

    let install = hooks_in(&dir, "install")
        .output()
        .unwrap_or_else(|e| panic!("run hooks install: {e:?}"));
    assert!(
        install.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&install.stderr)
    );

    let hook_path = dir.join(".git/hooks/pre-commit");
    let hook = std::fs::read_to_string(&hook_path).unwrap_or_else(|e| panic!("hook exists: {e:?}"));
    assert!(hook.contains("Installed by Pretender"), "hook: {hook}");
    assert!(
        hook.contains("exec pretender check . --staged"),
        "hook: {hook}"
    );

    let uninstall = hooks_in(&dir, "uninstall")
        .output()
        .unwrap_or_else(|e| panic!("run hooks uninstall: {e:?}"));
    assert!(
        uninstall.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&uninstall.stderr)
    );
    assert!(!hook_path.exists(), "hook should be removed");
}

#[test]
fn test_hooks_install_pre_push_installs_pre_push_hook() {
    let dir = tempdir();
    git_init(&dir);

    let install = hooks_in_named(&dir, ("install", "pre-push"))
        .output()
        .unwrap_or_else(|e| panic!("run hooks install pre-push: {e:?}"));
    assert!(
        install.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&install.stderr)
    );

    let hook_path = dir.join(".git/hooks/pre-push");
    let hook = std::fs::read_to_string(&hook_path)
        .unwrap_or_else(|e| panic!("pre-push hook exists: {e:?}"));
    assert!(hook.contains("Installed by Pretender"), "hook: {hook}");
    assert!(
        hook.contains("exec pretender check . --staged"),
        "hook: {hook}"
    );
    assert!(
        !dir.join(".git/hooks/pre-commit").exists(),
        "pre-commit untouched"
    );

    let uninstall = hooks_in_named(&dir, ("uninstall", "pre-push"))
        .output()
        .unwrap_or_else(|e| panic!("run hooks uninstall pre-push: {e:?}"));
    assert!(
        uninstall.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&uninstall.stderr)
    );
    assert!(!hook_path.exists(), "pre-push hook should be removed");
}

#[test]
fn test_hooks_install_creates_hooks_dir_when_absent() {
    let dir = tempdir();
    git_init(&dir);
    // .git/hooks does NOT exist
    std::fs::remove_dir_all(dir.join(".git/hooks"))
        .unwrap_or_else(|e| panic!("remove hooks dir: {e:?}"));

    let install = hooks_in(&dir, "install")
        .output()
        .unwrap_or_else(|e| panic!("run hooks install: {e:?}"));
    assert!(
        install.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&install.stderr)
    );
    assert!(
        dir.join(".git/hooks/pre-commit").exists(),
        "hook should exist even when .git/hooks was absent"
    );
}

#[test]
fn test_hooks_uninstall_is_noop_when_no_hook() {
    let dir = tempdir();
    git_init(&dir);
    // no hook file present

    let uninstall = hooks_in(&dir, "uninstall")
        .output()
        .unwrap_or_else(|e| panic!("run hooks uninstall: {e:?}"));
    assert!(
        uninstall.status.success(),
        "uninstall with no hook should succeed silently; stderr: {}",
        String::from_utf8_lossy(&uninstall.stderr)
    );
}

#[test]
fn test_hooks_uninstall_refuses_foreign_hook() {
    let dir = tempdir();
    git_init(&dir);
    let hooks_dir = dir.join(".git/hooks");
    std::fs::write(
        hooks_dir.join("pre-commit"),
        "#!/bin/sh\necho 'custom hook'\n",
    )
    .unwrap_or_else(|e| panic!("write foreign hook: {e:?}"));

    let uninstall = hooks_in(&dir, "uninstall")
        .output()
        .unwrap_or_else(|e| panic!("run hooks uninstall: {e:?}"));
    assert!(
        !uninstall.status.success(),
        "should refuse to remove a hook not installed by Pretender"
    );
    let stderr = String::from_utf8_lossy(&uninstall.stderr);
    assert!(
        stderr.contains("refusing to remove hook not installed by Pretender"),
        "stderr: {stderr}"
    );
    assert!(
        hooks_dir.join("pre-commit").exists(),
        "foreign hook should be left intact"
    );
}

#[test]
fn test_hooks_install_refuses_foreign_hook() {
    let dir = tempdir();
    git_init(&dir);
    let hooks_dir = dir.join(".git/hooks");
    std::fs::write(
        hooks_dir.join("pre-commit"),
        "#!/bin/sh\necho 'custom hook'\n",
    )
    .unwrap_or_else(|e| panic!("write foreign hook: {e:?}"));

    let install = hooks_in(&dir, "install")
        .output()
        .unwrap_or_else(|e| panic!("run hooks install: {e:?}"));
    assert!(
        !install.status.success(),
        "should refuse to overwrite a hook not installed by Pretender"
    );
    let stderr = String::from_utf8_lossy(&install.stderr);
    assert!(
        stderr.contains("refusing to overwrite hook not installed by Pretender"),
        "stderr: {stderr}"
    );
    let hook = std::fs::read_to_string(hooks_dir.join("pre-commit"))
        .unwrap_or_else(|e| panic!("hook still exists: {e:?}"));
    assert!(
        hook.contains("custom hook"),
        "foreign hook should be left intact: {hook}"
    );
}

#[test]
fn test_hooks_install_is_idempotent() {
    let dir = tempdir();
    git_init(&dir);

    let first = hooks_in(&dir, "install")
        .output()
        .unwrap_or_else(|e| panic!("first install: {e:?}"));
    assert!(
        first.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let second = hooks_in(&dir, "install")
        .output()
        .unwrap_or_else(|e| panic!("second install: {e:?}"));
    assert!(
        second.status.success(),
        "reinstall over Pretender-owned hook should succeed; stderr: {}",
        String::from_utf8_lossy(&second.stderr)
    );

    let hook = std::fs::read_to_string(dir.join(".git/hooks/pre-commit"))
        .unwrap_or_else(|e| panic!("hook exists: {e:?}"));
    assert!(
        hook.contains("exec pretender check . --staged"),
        "hook: {hook}"
    );
}

#[test]
fn test_hooks_install_from_subdirectory() {
    // Hooks must be installed at the repo root, not the CWD.
    let dir = tempdir();
    git_init(&dir);
    let subdir = dir.join("src/sub/deep");
    std::fs::create_dir_all(&subdir).unwrap_or_else(|e| panic!("create subdir: {e:?}"));

    let install = hooks_in(&subdir, "install")
        .output()
        .unwrap_or_else(|e| panic!("run hooks install from subdir: {e:?}"));
    assert!(
        install.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&install.stderr)
    );
    // Hook must be at repo root, not in the subdir
    assert!(
        dir.join(".git/hooks/pre-commit").exists(),
        "hook should be at repo root, not subdir"
    );
    assert!(
        !subdir.join(".git/hooks/pre-commit").exists(),
        "hook should NOT be created in subdirectory"
    );
}

#[test]
fn test_hooks_uninstall_outside_repo_fails_cleanly() {
    // Running hooks outside a git repo must fail clearly.
    let dir = tempdir();

    let uninstall = hooks_in(&dir, "uninstall")
        .output()
        .unwrap_or_else(|e| panic!("run hooks uninstall outside repo: {e:?}"));
    assert!(
        !uninstall.status.success(),
        "should fail outside a git repo"
    );
    let stderr = String::from_utf8_lossy(&uninstall.stderr);
    assert!(
        stderr.contains("git") || stderr.contains("repository"),
        "error should mention git/repository; got: {stderr}"
    );
}
