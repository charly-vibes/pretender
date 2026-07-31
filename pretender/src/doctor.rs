use crate::config;
use crate::external_plugin;
use anyhow::Result;
use genesis::config::ValidationSeverity;
use genesis::doctor::{DoctorCheck, DoctorReport, DoctorRunner};
use genesis::guide::OutputFormat;
use genesis::suite_linter::{LintResult, Severity};
use std::path::Path;
use std::process::ExitCode;

const PRE_COMMIT_HOOK_MARKER: &str = "# Installed by Pretender.";

// ── DoctorCheck implementations ───────────────────────────────────────

struct GitContextCheck;
impl DoctorCheck for GitContextCheck {
    fn name(&self) -> &'static str {
        "Git context"
    }
    fn description(&self) -> &'static str {
        "Check that the working directory is inside a git repository"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if repo_root.join(".git").exists() {
            Ok(vec![])
        } else {
            Ok(vec![LintResult::new(
                "not inside a git repository",
                Severity::Error,
            )])
        }
    }
}

struct ConfigPresentCheck;
impl DoctorCheck for ConfigPresentCheck {
    fn name(&self) -> &'static str {
        "Config present"
    }
    fn description(&self) -> &'static str {
        "Check that pretender.toml exists in the current directory"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if repo_root.join("pretender.toml").exists() {
            Ok(vec![])
        } else {
            Ok(vec![LintResult::new(
                "pretender.toml not found in current directory",
                Severity::Error,
            )])
        }
    }
}

struct ConfigValidCheck;
impl DoctorCheck for ConfigValidCheck {
    fn name(&self) -> &'static str {
        "Config valid"
    }
    fn description(&self) -> &'static str {
        "Validate pretender.toml configuration"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if !repo_root.join("pretender.toml").exists() {
            return Ok(vec![LintResult::new(
                "skipped (config not present)",
                Severity::Warning,
            )]);
        }
        let store = genesis::config::ConfigStore::new(config::build_registry());
        let validations = store.validate_all(repo_root);
        let first_error = validations
            .iter()
            .find(|v| v.severity == ValidationSeverity::Error);
        match first_error {
            None => Ok(vec![]),
            Some(err) => Ok(vec![LintResult::new(err.message.clone(), Severity::Error)]),
        }
    }
}

struct HookInstalledCheck;
impl DoctorCheck for HookInstalledCheck {
    fn name(&self) -> &'static str {
        "Hook installed"
    }
    fn description(&self) -> &'static str {
        "Check that the pre-commit hook is installed"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let git_dir = repo_root.join(".git");
        if !git_dir.exists() {
            return Ok(vec![LintResult::new(
                "skipped (not in a git repository)",
                Severity::Warning,
            )]);
        }
        let hook_path = repo_root.join(".git/hooks/pre-commit");
        match std::fs::read_to_string(&hook_path) {
            Ok(content) if content.contains(PRE_COMMIT_HOOK_MARKER) => Ok(vec![]),
            Ok(_) => Ok(vec![LintResult::new(
                "pre-commit hook exists but is not managed by Pretender",
                Severity::Error,
            )]),
            Err(_) => Ok(vec![LintResult::new(
                "pre-commit hook not found at .git/hooks/pre-commit",
                Severity::Error,
            )]),
        }
    }
}

struct HookExecutableCheck;
impl DoctorCheck for HookExecutableCheck {
    fn name(&self) -> &'static str {
        "Hook executable"
    }
    fn description(&self) -> &'static str {
        "Check that the pre-commit hook is executable"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let hook_path = repo_root.join(".git/hooks/pre-commit");
        if !hook_path.exists() {
            return Ok(vec![LintResult::new(
                "skipped (hook not installed)",
                Severity::Warning,
            )]);
        }
        // Only check executability for pretender-managed hooks
        let content = std::fs::read_to_string(&hook_path).unwrap_or_default();
        if !content.contains(PRE_COMMIT_HOOK_MARKER) {
            return Ok(vec![LintResult::new(
                "skipped (hook not managed by Pretender)",
                Severity::Warning,
            )]);
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            match std::fs::metadata(&hook_path) {
                Ok(meta) if meta.permissions().mode() & 0o111 != 0 => Ok(vec![]),
                Ok(_) => Ok(vec![LintResult::new(
                    "hook file is not executable",
                    Severity::Error,
                )]),
                Err(e) => Ok(vec![LintResult::new(
                    format!("could not read hook metadata: {e}"),
                    Severity::Error,
                )]),
            }
        }
        #[cfg(windows)]
        {
            // Windows uses ACLs, not Unix permission bits.
            Ok(vec![])
        }
    }
}

struct PluginManifestsCheck;
impl DoctorCheck for PluginManifestsCheck {
    fn name(&self) -> &'static str {
        "Plugin manifests"
    }
    fn description(&self) -> &'static str {
        "Validate plugin manifest files"
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let dir = external_plugin::default_metrics_dir();
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return Ok(vec![LintResult::new(
                "no external metrics directory configured",
                Severity::Warning,
            )]);
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
            Ok(vec![])
        } else {
            Ok(vec![LintResult::new(
                format!("invalid plugin manifests: {}", invalid.join(", ")),
                Severity::Error,
            )])
        }
    }
}

// ── Public API ────────────────────────────────────────────────────────

pub fn run_doctor(format: OutputFormat) -> Result<ExitCode> {
    let runner = DoctorRunner::new(vec![
        Box::new(GitContextCheck),
        Box::new(ConfigPresentCheck),
        Box::new(ConfigValidCheck),
        Box::new(HookInstalledCheck),
        Box::new(HookExecutableCheck),
        Box::new(PluginManifestsCheck),
    ])
    .with_tool_name("pretender");

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let report = runner
        .run(&cwd, false)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    match format {
        OutputFormat::Human => print_human(&report),
        OutputFormat::Json => print_json(&report)?,
    }

    Ok(if report.summary.has_failures() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

fn print_human(report: &DoctorReport) {
    for check in &report.checks {
        let prefix = match check.status {
            genesis::doctor::CheckStatus::Pass => "✓",
            genesis::doctor::CheckStatus::Warn => "⚠",
            genesis::doctor::CheckStatus::Fail => "✗",
        };
        println!("{prefix} {} — {}", check.name, check.message);
    }
    let total = report.checks.len();
    let passed = report.summary.pass;
    println!("\n{passed}/{total} checks passed");
}

fn print_json(report: &DoctorReport) -> Result<()> {
    let envelope = report.to_envelope();
    let json = serde_json::to_string_pretty(&envelope)?;
    println!("{json}");
    Ok(())
}
