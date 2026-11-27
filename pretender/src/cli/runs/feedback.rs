//! Runner for the `feedback` subcommand: repository extraction and issue
//! creation through genesis.
//!
//! # Responsibilities
//! `FeedbackArgs::run`, the genesis outcome-to-exit-code translation,
//! the CLI-to-genesis kind mapping, the issue-outcome printer, and the
//! repository resolution (URL trimming in `repo_slug`): git remote
//! `origin` first, then Cargo.toml's [package].repository, then the
//! `unknown/unknown` placeholder.
//!
//! # Rationale
//! Split out of `cli/runs.rs` (pretender-5zo.15.14) along the feedback
//! seam so every file stays under the file-size gate. Bodies moved
//! verbatim except where a function had to split further (`run`'s outcome
//! match became `feedback_exit_code`, `extract_repo_from_cargo_toml`'s
//! URL trimming became `repo_slug`) with output unchanged.
use crate::cli::args::{FeedbackArgs, FeedbackKind};
use crate::cli::Executable;
use anyhow::{Context, Result};
use genesis::feedback::{self as genesis_feedback, FeedbackArgs as GenesisFeedbackArgs};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

impl Executable for FeedbackArgs {
    fn run(&self) -> Result<ExitCode> {
        let repo = extract_repo().unwrap_or_else(|| "unknown/unknown".to_string());
        let project_root = std::env::current_dir().context("failed to get current directory")?;

        let genesis_args = GenesisFeedbackArgs::new(
            feedback_kind_str(self.kind),
            self.dry_run,
            self.from_last_error,
        );
        feedback_exit_code(genesis_feedback::handle_feedback(
            &genesis_args,
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            &repo,
            &project_root,
        ))
    }
}

/// Translate the genesis feedback outcome into a process exit code,
/// printing the created issue or the failure message.
fn feedback_exit_code(result: Result<genesis_feedback::gh::GhResult, String>) -> Result<ExitCode> {
    match result {
        Ok(result) => {
            print_feedback_result(result);
            Ok(ExitCode::SUCCESS)
        }
        Err(msg) => {
            eprintln!("Failed to create issue: {msg}");
            Ok(ExitCode::FAILURE)
        }
    }
}

/// Map the CLI feedback kind to the genesis kind string.
fn feedback_kind_str(kind: FeedbackKind) -> &'static str {
    match kind {
        FeedbackKind::Bug => "bug",
        FeedbackKind::Feature => "feature",
        FeedbackKind::Question => "question",
    }
}

/// Print the issue-creation outcome (URL, number, or local fallback path).
fn print_feedback_result(result: genesis::feedback::gh::GhResult) {
    match result {
        genesis::feedback::gh::GhResult::Created { url, number } => {
            println!("Created issue #{number}: {url}");
        }
        genesis::feedback::gh::GhResult::FallbackUrl(url) => {
            println!("Open to create issue: {url}");
        }
        genesis::feedback::gh::GhResult::LocalFile(path) => {
            println!("Issue body written to: {}", path.display());
        }
    }
}

/// Resolve the repository string: prefer the git `origin` remote, then
/// fall back to Cargo.toml's [package].repository; `None` when neither
/// is available (callers substitute a placeholder).
fn extract_repo() -> Option<String> {
    let remote_url = std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|url| !url.is_empty());
    resolve_repo(remote_url)
}

/// Testable core of `extract_repo`: normalize a git remote URL if given,
/// otherwise fall back to Cargo.toml, mapping failures to `None`.
fn resolve_repo(remote_url: Option<String>) -> Option<String> {
    let project_root = std::env::current_dir()
        .context("failed to get current directory")
        .ok()?;
    resolve_repo_at(remote_url, &project_root)
}

/// Testable core of `resolve_repo` that never touches the process cwd:
/// the Cargo.toml fallback reads `project_root`'s manifest instead.
fn resolve_repo_at(remote_url: Option<String>, project_root: &Path) -> Option<String> {
    remote_url
        .map(|url| repo_slug(&url))
        .or_else(|| extract_repo_from_cargo_toml_at(project_root).ok())
}

/// Extract the repository string from `project_root`'s Cargo.toml
/// ([package] repository field).
/// Prefer the pretender member's Cargo.toml over the workspace root's.
fn manifest_path(project_root: &Path) -> PathBuf {
    let member_path = project_root.join("pretender/Cargo.toml");
    if member_path.exists() {
        member_path
    } else {
        project_root.join("Cargo.toml")
    }
}

fn extract_repo_from_cargo_toml_at(project_root: &Path) -> Result<String> {
    let cargo_path = manifest_path(project_root);
    let cargo_path = cargo_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Cargo.toml path is not valid UTF-8"))?;
    let content =
        std::fs::read_to_string(cargo_path).context(format!("failed to read {cargo_path}"))?;
    let value: toml::Value = toml::from_str(&content).context("failed to parse Cargo.toml")?;
    let package = value
        .get("package")
        .and_then(|v| v.get("repository"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Cargo.toml missing [package].repository"))?;
    // Convert full URL to owner/repo format
    Ok(repo_slug(package))
}

/// Convert a repository URL to `owner/repo` form.
///
/// Canonicalization (protocol, credentials, scp syntax, query string,
/// `.git` suffix) delegates to [`genesis::git::remote_key`]; the leading
/// host segment is dropped because genesis feedback files issues against
/// `owner/repo` (`gh issue create --repo`, fallback issue URLs).
///
/// Handles: https://github.com/owner/repo, https://github.com/owner/repo.git,
///          git@github.com:owner/repo.git, ssh://git@github.com/owner/repo.git,
///          plus PAT (`https://pat@github.com/...`) and query-string forms
fn repo_slug(url: &str) -> String {
    let key = genesis::git::remote_key(url);
    match key.split_once('/') {
        Some((_host, owner_repo)) => owner_repo.to_string(),
        None => key,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The genesis `remote_key` fixture set (genesis
    /// tests/git_queries_remote.rs): `(url, remote_key output, repo_slug
    /// output)` — the slug is the key minus its host segment.
    const REMOTE_KEY_FIXTURES: &[(&str, &str, &str)] = &[
        (
            "git@github.com:owner/repo.git",
            "github.com/owner/repo",
            "owner/repo",
        ),
        (
            "ssh://git@github.com/owner/repo.git",
            "github.com/owner/repo",
            "owner/repo",
        ),
        (
            "https://github.com/owner/repo.git",
            "github.com/owner/repo",
            "owner/repo",
        ),
        (
            "https://pat@github.com/owner/repo.git",
            "github.com/owner/repo",
            "owner/repo",
        ),
        (
            "https://github.com/owner/repo",
            "github.com/owner/repo",
            "owner/repo",
        ),
        (
            "https://github.com/owner/repo?foo=bar",
            "github.com/owner/repo",
            "owner/repo",
        ),
        (
            "http://user:pass@host.tld/org/repo.git",
            "host.tld/org/repo",
            "org/repo",
        ),
    ];

    #[test]
    fn repo_slug_strips_common_remotes() {
        for url in [
            "https://github.com/o/r",
            "https://github.com/o/r.git",
            "git@github.com:o/r.git",
            "ssh://git@github.com/o/r.git",
        ] {
            assert_eq!(repo_slug(url), "o/r");
        }
    }

    #[test]
    fn repo_slug_matches_remote_key_on_genesis_fixtures() {
        // The genesis `remote_key` fixture set (genesis
        // tests/git_queries_remote.rs): scp, ssh, https, PAT, and
        // query-string forms. repo_slug must stay equivalent to
        // `genesis::git::remote_key` with the host segment dropped
        // (genesis feedback files against `owner/repo`), and byte-identical
        // to the pre-migration slug on the github forms.
        for &(url, key, slug) in REMOTE_KEY_FIXTURES {
            // Byte-identical to genesis's own remote_key output.
            assert_eq!(genesis::git::remote_key(url), key, "remote_key({url})");
            // repo_slug ≡ remote_key with the host segment dropped.
            let owner_repo = key.split_once('/').map(|(_, rest)| rest.to_string());
            assert_eq!(
                repo_slug(url),
                owner_repo.unwrap_or_default(),
                "slug({url})"
            );
            assert_eq!(repo_slug(url), slug, "slug({url})");
        }
    }

    #[test]
    fn extract_repo_from_cargo_toml_errors_without_manifest() {
        let result = extract_repo_from_cargo_toml_at(Path::new("/nonexistent-empty-dir-gh91"));
        assert!(result.is_err());
    }

    #[test]
    fn resolve_repo_returns_none_without_remote_or_manifest() {
        let result = resolve_repo_at(None, Path::new("/nonexistent-empty-dir-gh91"));
        assert_eq!(result, None);
    }

    #[test]
    fn resolve_repo_prefers_remote() {
        assert_eq!(
            resolve_repo(Some("git@github.com:o/r.git".to_string())),
            Some("o/r".to_string())
        );
    }
}
