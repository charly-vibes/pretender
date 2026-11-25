//! `doctor` subcommand tests: check/skip reporting and envelope shapes.
//!
//! # Responsibilities
//! Pin doctor's human and JSON output — envelope `ok`/`envelope_version`/
//! `envelope_kind` shape, Config-present failure and downstream skips,
//! unmanaged-hook failure with executable skip, and the Hook-location check
//! under a redirected versus default `core.hooksPath`.
//!
//! # Rationale
//! Moved verbatim from `tests/cli_test.rs` (wave 3 per-command split);
//! expect/unwrap calls converted to the sanctioned panic-on-failure forms.

use crate::common::*;

use std::path::Path;

/// Write a Pretender-managed pre-commit hook (marked + executable) into
/// `dir`'s `.git/hooks`, as `hooks install` would leave it.
fn write_pretender_hook(dir: &Path) {
    let hooks_dir = dir.join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).unwrap_or_else(|e| panic!("create hooks dir: {e:?}"));
    let hook = hooks_dir.join("pre-commit");
    std::fs::write(
        &hook,
        "#!/usr/bin/env sh\n# Installed by Pretender.\nexec pretender check . --staged\n",
    )
    .unwrap_or_else(|e| panic!("write hook: {e:?}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
            .unwrap_or_else(|e| panic!("chmod hook: {e:?}"));
    }
}

#[test]
fn test_doctor_json_output_has_envelope_shape() {
    let dir = tempdir();
    git_init(&dir);

    let output = doctor_in(&dir, &["--json"])
        .output()
        .unwrap_or_else(|e| panic!("run doctor: {e:?}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("output should be valid JSON: {e:?}"));

    // Verify genesis envelope shape
    assert!(json.get("ok").is_some(), "envelope must have 'ok' field");
    assert_eq!(json["ok"], true, "doctor ran successfully, ok=true");
    assert_eq!(
        json["envelope_version"], "0.1",
        "envelope_version should be 0.1"
    );
    assert_eq!(
        json["envelope_kind"], "doctor",
        "envelope_kind should be 'doctor'"
    );
    assert!(
        json.get("data").is_some(),
        "envelope must have 'data' field"
    );
    // Verify existing data is preserved within the envelope
    let checks = json["data"]["checks"]
        .as_array()
        .unwrap_or_else(|| panic!("data.checks should be array"));
    let config_present = checks
        .iter()
        .find(|c| c["name"] == "Config present")
        .unwrap_or_else(|| panic!("Config present check missing"));
    assert_eq!(
        config_present["status"], "fail",
        "Config present should have status=fail"
    );
}

#[test]
fn test_doctor_all_pass_with_valid_config_and_hook() {
    let dir = tempdir();
    git_init(&dir);
    std::fs::write(dir.join("pretender.toml"), "[pretender]\n")
        .unwrap_or_else(|e| panic!("write config: {e:?}"));
    write_pretender_hook(&dir);

    let output = doctor_in(&dir, &["--human"])
        .output()
        .unwrap_or_else(|e| panic!("run doctor: {e:?}"));

    assert_eq!(
        output.status.code(),
        Some(0),
        "doctor should exit 0; stdout: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("checks passed"),
        "expected summary; got: {stdout}"
    );
}

#[test]
fn test_doctor_missing_config_exits_1_with_correct_skips() {
    let dir = tempdir();
    git_init(&dir);

    let output = doctor_in(&dir, &["--human"])
        .output()
        .unwrap_or_else(|e| panic!("run doctor: {e:?}"));

    assert_eq!(output.status.code(), Some(1), "should exit 1");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("✗ Config present"),
        "Config present should fail; got: {stdout}"
    );
    assert!(
        stdout.contains("⚠ Config valid"),
        "Config valid should be skipped; got: {stdout}"
    );
    assert!(
        stdout.contains("⚠ Plugin manifests"),
        "Plugin manifests should be skipped; got: {stdout}"
    );
}

#[test]
fn test_doctor_json_missing_config_exits_1_with_fail_status() {
    let dir = tempdir();
    git_init(&dir);

    let output = doctor_in(&dir, &["--json"])
        .output()
        .unwrap_or_else(|e| panic!("run doctor: {e:?}"));

    assert_eq!(output.status.code(), Some(1), "should exit 1");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("output should be valid JSON: {e:?}"));
    let checks = parsed["data"]["checks"]
        .as_array()
        .unwrap_or_else(|| panic!("JSON data.checks should be array"));
    let config_present = checks
        .iter()
        .find(|c| c["name"] == "Config present")
        .unwrap_or_else(|| panic!("Config present check missing"));
    assert_eq!(
        config_present["status"], "fail",
        "Config present should have status=fail"
    );
}

#[test]
fn test_doctor_unmanaged_hook_fails_installed_skips_executable() {
    let dir = tempdir();
    git_init(&dir);
    std::fs::write(dir.join("pretender.toml"), "[pretender]\n")
        .unwrap_or_else(|e| panic!("write config: {e:?}"));
    let hooks_dir = dir.join(".git/hooks");
    std::fs::create_dir_all(&hooks_dir).unwrap_or_else(|e| panic!("create hooks dir: {e:?}"));
    let hook = hooks_dir.join("pre-commit");
    std::fs::write(&hook, "#!/usr/bin/env sh\necho 'custom hook'\n")
        .unwrap_or_else(|e| panic!("write hook: {e:?}"));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
            .unwrap_or_else(|e| panic!("chmod hook: {e:?}"));
    }

    let output = doctor_in(&dir, &["--human"])
        .output()
        .unwrap_or_else(|e| panic!("run doctor: {e:?}"));

    assert_eq!(output.status.code(), Some(1), "should exit 1");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("✗ Hook installed"),
        "Hook installed should fail; got: {stdout}"
    );
    assert!(
        stdout.contains("⚠ Hook executable"),
        "Hook executable should be skipped; got: {stdout}"
    );
}

#[test]
fn test_doctor_warns_when_global_hooks_path_redirects_hook() {
    let dir = tempdir();
    git_init(&dir);
    std::fs::write(dir.join("pretender.toml"), "[pretender]\n")
        .unwrap_or_else(|e| panic!("write config: {e:?}"));
    write_pretender_hook(&dir);

    // Global git config points core.hooksPath elsewhere (the lefthook-shim
    // scenario): git will invoke hooks from the redirected dir, so the
    // installed .git/hooks/pre-commit is inert.
    let global_cfg = dir.join("redirect.gitconfig");
    std::fs::write(
        &global_cfg,
        "[core]\n\thooksPath = /nonexistent-shim-hooks\n",
    )
    .unwrap_or_else(|e| panic!("write redirect gitconfig: {e:?}"));

    let mut cmd = pretender_cmd();
    cmd.arg("doctor")
        .arg("--human")
        .current_dir(&dir)
        .env("NO_COLOR", "1")
        .env("GIT_CONFIG_GLOBAL", &global_cfg);
    let output = cmd.output().unwrap_or_else(|e| panic!("run doctor: {e:?}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hook location"),
        "expected a 'Hook location' doctor check; got: {stdout}"
    );
    assert!(
        stdout.contains("core.hooksPath"),
        "warning should mention core.hooksPath; got: {stdout}"
    );
    assert!(
        stdout.contains("global"),
        "warning should name the config scope; got: {stdout}"
    );
}

#[test]
fn test_doctor_hook_location_passes_when_hooks_path_default() {
    let dir = tempdir();
    git_init(&dir);
    std::fs::write(dir.join("pretender.toml"), "[pretender]\n")
        .unwrap_or_else(|e| panic!("write config: {e:?}"));
    write_pretender_hook(&dir);

    let output = doctor_in(&dir, &["--human"])
        .output()
        .unwrap_or_else(|e| panic!("run doctor: {e:?}"));

    let stdout = String::from_utf8_lossy(&output.stdout);
    // The check runs but must not warn under the default (no redirect).
    assert!(
        stdout.contains("✓ Hook location"),
        "hook-location check should pass with default hooksPath; got: {stdout}"
    );
}
