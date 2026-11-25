//! Check step helpers split out of the `DoctorCheck::run` methods to keep
//! each within the gate's structural budgets. Bodies are moved verbatim;
//! signatures of the checks themselves are unchanged.

use genesis::suite_linter::{LintResult, Severity};
use std::path::Path;

/// Executability verdict for an installed, Pretender-managed hook. Split
/// out of `HookExecutableCheck::run`; the platform-gated metadata check
/// dominates that method's length.
#[cfg_attr(windows, allow(unused_variables))]
pub(super) fn hook_executability_result(
    hook_path: &Path,
) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        unix_executability(hook_path)
    }
    #[cfg(windows)]
    {
        // Windows uses ACLs, not Unix permission bits.
        Ok(vec![])
    }
}

/// Unix permission-bit check for the hook file.
#[cfg(unix)]
fn unix_executability(hook_path: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
    use std::os::unix::fs::PermissionsExt;
    match std::fs::metadata(hook_path) {
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

/// Warning matrix for git's effective `core.hooksPath` when a
/// Pretender-managed hook is installed under `.git/hooks`. Split out of
/// `HookLocationCheck::run`; the per-scope diagnostics dominate that
/// method's length.
pub(super) fn hooks_path_scope_result(
    repo_root: &Path,
    effective: genesis::git_hooks::EffectiveHooksDir,
) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
    use genesis::git_hooks::HooksDirScope;
    match &effective.scope {
        HooksDirScope::Default => Ok(vec![]),
        HooksDirScope::Disabled => Ok(vec![hooks_disabled_warning()]),
        scope => Ok(vec![hooks_redirected_warning(repo_root, &effective, scope)]),
    }
}

/// Error for `core.hooksPath = ""`, which makes git disable hooks entirely.
fn hooks_disabled_warning() -> LintResult {
    LintResult::new(
        "core.hooksPath is set to the empty string — git disables hooks entirely, \
         so the installed pretender pre-commit hook is inert. \
         Remove the core.hooksPath setting to re-enable the gate.",
        Severity::Error,
    )
}

/// Warning for a `core.hooksPath` that points git at a directory other than
/// the one holding the installed Pretender hook.
fn hooks_redirected_warning(
    repo_root: &Path,
    effective: &genesis::git_hooks::EffectiveHooksDir,
    scope: &genesis::git_hooks::HooksDirScope,
) -> LintResult {
    LintResult::new(
        format!(
            "git will invoke hooks from {} (core.hooksPath set at {} scope), \
             but the pretender hook is installed at {} — the gate is inert. \
             Remove the core.hooksPath setting or install the hook in the \
             effective directory.",
            effective.path.display(),
            format!("{scope:?}").to_lowercase(),
            repo_root.join(".git/hooks/pre-commit").display()
        ),
        Severity::Warning,
    )
}

/// Names of `*.toml` files in the external metrics directory whose contents
/// do not parse. Split out of `PluginManifestsCheck::run`; the directory
/// scan dominates that method's length.
pub(super) fn invalid_manifest_names(entries: std::fs::ReadDir) -> Vec<String> {
    entries
        .flatten()
        .filter(|entry| is_invalid_manifest(&entry.path()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect()
}

/// Whether a directory entry is a `*.toml` file whose contents fail to parse.
fn is_invalid_manifest(path: &Path) -> bool {
    if path.extension().and_then(|s| s.to_str()) != Some("toml") {
        return false;
    }
    let Ok(source) = std::fs::read_to_string(path) else {
        return false;
    };
    toml::from_str::<toml::Value>(&source).is_err()
}
