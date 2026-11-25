//! The `DoctorCheck` implementations executed by the doctor runner.
//!
//! Responsibilities: one self-contained check type per health dimension
//! (git context, config presence/validity, hook installation,
//! executability, location, hooks-vs-config drift, plugin manifests).
//! Rationale: bodies stay verbatim from the pre-split module; shared
//! step helpers live in `steps`.

use crate::config;
use crate::external_plugin;
use genesis::config::ValidationSeverity;
use genesis::doctor::DoctorCheck;
use genesis::suite_linter::{LintResult, Severity};
use std::path::Path;

use super::steps::{hook_executability_result, hooks_path_scope_result, invalid_manifest_names};
use super::PRE_COMMIT_HOOK_MARKER;

pub(super) struct GitContextCheck;
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

pub(super) struct ConfigPresentCheck;
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

pub(super) struct ConfigValidCheck;
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

pub(super) struct HookInstalledCheck;
impl DoctorCheck for HookInstalledCheck {
    fn name(&self) -> &'static str {
        "Hook installed"
    }
    fn description(&self) -> &'static str {
        "Check that the pre-commit hook is installed"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if !repo_root.join(".git").exists() {
            return Ok(vec![LintResult::new(
                "skipped (not in a git repository)",
                Severity::Warning,
            )]);
        }
        let hook_path = repo_root.join(".git/hooks/pre-commit");
        Ok(hook_install_verdict(&hook_path))
    }
}

pub(super) struct HookExecutableCheck;
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
        hook_executability_result(&hook_path)
    }
}

pub(super) struct HooksConfigMismatchCheck;
impl DoctorCheck for HooksConfigMismatchCheck {
    fn name(&self) -> &'static str {
        "Hooks vs config"
    }
    fn description(&self) -> &'static str {
        "Warn when the pre-commit hook is installed but no pretender.toml exists"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let hook_path = repo_root.join(".git/hooks/pre-commit");
        // Only relevant when a Pretender-managed hook file exists
        if managed_hook_content(&hook_path).is_none() {
            return Ok(vec![]);
        }
        if repo_root.join("pretender.toml").exists() {
            return Ok(vec![]); // both hook and config present → happy path
        }
        Ok(vec![hooks_without_config_warning(&hook_path)])
    }
}

pub(super) struct HookLocationCheck;
impl DoctorCheck for HookLocationCheck {
    fn name(&self) -> &'static str {
        "Hook location"
    }
    fn description(&self) -> &'static str {
        "Warn when core.hooksPath redirects hooks away from the installed pretender hook"
    }
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if !repo_root.join(".git").exists() {
            return Ok(vec![LintResult::new(
                "skipped (not in a git repository)",
                Severity::Warning,
            )]);
        }
        // Only relevant when a pretender-managed hook is installed in .git/hooks
        let hook_content =
            std::fs::read_to_string(repo_root.join(".git/hooks/pre-commit")).unwrap_or_default();
        if !hook_content.contains(PRE_COMMIT_HOOK_MARKER) {
            return Ok(vec![]);
        }
        let effective = genesis::git_hooks::effective_hooks_dir(repo_root)?;
        hooks_path_scope_result(repo_root, effective)
    }
}

pub(super) struct PluginManifestsCheck;
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
        let invalid = invalid_manifest_names(entries);
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

/// Verdict for the installed pre-commit hook: Pretender-managed → clean,
/// foreign → error, missing → error.
fn hook_install_verdict(hook_path: &Path) -> Vec<LintResult> {
    match std::fs::read_to_string(hook_path) {
        Ok(content) if content.contains(PRE_COMMIT_HOOK_MARKER) => vec![],
        Ok(_) => vec![LintResult::new(
            "pre-commit hook exists but is not managed by Pretender",
            Severity::Error,
        )],
        Err(_) => vec![LintResult::new(
            "pre-commit hook not found at .git/hooks/pre-commit",
            Severity::Error,
        )],
    }
}

/// Content of the hook file when it exists and is Pretender-managed;
/// `None` for a missing, unreadable, or foreign hook (nothing to warn about).
fn managed_hook_content(hook_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(hook_path).ok()?;
    content.contains(PRE_COMMIT_HOOK_MARKER).then_some(content)
}

/// Drift warning raised when a Pretender-managed hook is installed but no
/// `pretender.toml` config exists alongside it.
fn hooks_without_config_warning(hook_path: &Path) -> LintResult {
    LintResult::new(
        format!(
            "pretender hook is installed ({}) but no pretender.toml found — \
             pretender check runs with library defaults, which may not match \
             your project's role layout and quality targets. \
             Run `pretender init` to generate a config, or copy \
             templates/pretender.toml.example from the pretender repo.",
            hook_path.display()
        ),
        Severity::Warning,
    )
}
