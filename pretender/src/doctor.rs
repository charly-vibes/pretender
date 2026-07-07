use crate::config::Config;
use crate::external_plugin;
use crate::DoctorFormat;
use anyhow::Result;
use serde::Serialize;
use std::process::{Command, ExitCode};

const PRE_COMMIT_HOOK_MARKER: &str = "# Installed by Pretender.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Fail,
    Skip,
}

#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub name: &'static str,
    pub status: CheckStatus,
    pub message: String,
}

fn pass(name: &'static str, message: impl Into<String>) -> CheckResult {
    CheckResult {
        name,
        status: CheckStatus::Pass,
        message: message.into(),
    }
}

fn fail(name: &'static str, message: impl Into<String>) -> CheckResult {
    CheckResult {
        name,
        status: CheckStatus::Fail,
        message: message.into(),
    }
}

fn skip(name: &'static str, message: impl Into<String>) -> CheckResult {
    CheckResult {
        name,
        status: CheckStatus::Skip,
        message: message.into(),
    }
}

pub fn run_doctor(format: DoctorFormat) -> Result<ExitCode> {
    let results = run_checks();
    match format {
        DoctorFormat::Human => print_human(&results),
        DoctorFormat::Json => print_json(&results)?,
    }
    let any_failed = results.iter().any(|r| r.status == CheckStatus::Fail);
    Ok(if any_failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn run_checks() -> Vec<CheckResult> {
    let git = check_git_context();
    let git_pass = git.status == CheckStatus::Pass;

    let config_present = check_config_present();
    let config_present_pass = config_present.status == CheckStatus::Pass;

    let config_valid = if config_present_pass {
        check_config_valid()
    } else {
        skip("Config valid", "skipped (config not present)")
    };

    let hook_installed = if git_pass {
        check_hook_installed()
    } else {
        skip("Hook installed", "skipped (not in a git repository)")
    };
    let hook_installed_pass = hook_installed.status == CheckStatus::Pass;

    let hook_executable = if !git_pass {
        skip("Hook executable", "skipped (not in a git repository)")
    } else if !hook_installed_pass {
        skip("Hook executable", "skipped (hook not installed)")
    } else {
        check_hook_executable()
    };

    let plugin_manifests = if config_present_pass {
        check_plugin_manifests()
    } else {
        skip("Plugin manifests", "skipped (config not present)")
    };

    vec![
        git,
        config_present,
        config_valid,
        hook_installed,
        hook_executable,
        plugin_manifests,
    ]
}

fn check_git_context() -> CheckResult {
    let ok = Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok {
        pass(
            "Git context",
            "working directory is inside a git repository",
        )
    } else {
        fail("Git context", "not inside a git repository")
    }
}

fn check_config_present() -> CheckResult {
    if std::path::Path::new("pretender.toml").exists() {
        pass("Config present", "pretender.toml found")
    } else {
        fail(
            "Config present",
            "pretender.toml not found in current directory",
        )
    }
}

fn check_config_valid() -> CheckResult {
    match Config::load_from_path("pretender.toml") {
        Ok(_) => pass("Config valid", "configuration is valid"),
        Err(e) => fail("Config valid", e.to_string()),
    }
}

fn check_hook_installed() -> CheckResult {
    let hook_path = std::path::Path::new(".git/hooks/pre-commit");
    match std::fs::read_to_string(hook_path) {
        Ok(content) if content.contains(PRE_COMMIT_HOOK_MARKER) => pass(
            "Hook installed",
            "Pretender-managed pre-commit hook is installed",
        ),
        Ok(_) => fail(
            "Hook installed",
            "pre-commit hook exists but is not managed by Pretender",
        ),
        Err(_) => fail(
            "Hook installed",
            "pre-commit hook not found at .git/hooks/pre-commit",
        ),
    }
}

fn check_hook_executable() -> CheckResult {
    use std::os::unix::fs::PermissionsExt;
    let hook_path = std::path::Path::new(".git/hooks/pre-commit");
    match std::fs::metadata(hook_path) {
        Ok(meta) if meta.permissions().mode() & 0o111 != 0 => {
            pass("Hook executable", "hook file has executable permission")
        }
        Ok(_) => fail("Hook executable", "hook file is not executable"),
        Err(e) => fail(
            "Hook executable",
            format!("could not read hook metadata: {e}"),
        ),
    }
}

fn check_plugin_manifests() -> CheckResult {
    let dir = external_plugin::default_metrics_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return pass(
            "Plugin manifests",
            "no external metrics directory configured",
        );
    };
    let mut invalid: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        if toml::from_str::<toml::Value>(&source).is_err() {
            invalid.push(
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    if invalid.is_empty() {
        pass("Plugin manifests", "all plugin manifests are valid")
    } else {
        fail(
            "Plugin manifests",
            format!("invalid plugin manifests: {}", invalid.join(", ")),
        )
    }
}

fn print_human(results: &[CheckResult]) {
    for r in results {
        let prefix = match r.status {
            CheckStatus::Pass => "✓",
            CheckStatus::Fail => "✗",
            CheckStatus::Skip => "⚠",
        };
        println!("{prefix} {} — {}", r.name, r.message);
    }
    let passed = results
        .iter()
        .filter(|r| r.status == CheckStatus::Pass)
        .count();
    println!("\n{passed}/6 checks passed");
}

fn print_json(results: &[CheckResult]) -> Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    println!("{json}");
    Ok(())
}
