//! Hooks and CI subcommands: git hook install/uninstall and workflow
//! generation.
//!
//! Implements `pretender hooks` and `pretender ci generate` on top of the
//! genesis git-hooks helpers, including the foreign-hook refusal errors.
//!
//! # Responsibilities
//! The `HooksCommand` and `CiCommand` runners plus their plumbing: hook
//! script text, install/uninstall wrappers, error messaging, repo-root
//! lookup, and the GitHub CI workflow writer.
//!
//! # Rationale
//! Extracted verbatim from `main.rs` (pretender-5zo.2.5); `install_hook`
//! and `write_github_ci_workflow` are shared with the `init` pipeline,
//! so they stay `pub(crate)`.
use crate::cli::args::{CiCommand, CiProvider, HooksCommand};
use crate::cli::{not_implemented, Executable};
use crate::init::github_ci_workflow_contents;
use anyhow::{anyhow, Context, Result};
use genesis::git_hooks::{self as genesis_git_hooks, HookName};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

impl Executable for HooksCommand {
    fn run(&self) -> Result<ExitCode> {
        match self {
            HooksCommand::Install { hook } => {
                install_hook((*hook).into())?;
                Ok(ExitCode::SUCCESS)
            }
            HooksCommand::Uninstall { hook } => {
                uninstall_hook((*hook).into())?;
                Ok(ExitCode::SUCCESS)
            }
        }
    }
}

impl Executable for CiCommand {
    fn run(&self) -> Result<ExitCode> {
        match self {
            CiCommand::Generate {
                provider: CiProvider::Github,
            } => {
                write_github_ci_workflow()?;
                Ok(ExitCode::SUCCESS)
            }
            CiCommand::Generate { .. } => not_implemented("ci generate", "pretender-fb3"),
        }
    }
}

const PRE_COMMIT_HOOK_MARKER: &str = "# Installed by Pretender.";

fn hook_script() -> String {
    "#!/usr/bin/env sh\nexec pretender check . --staged\n".to_string()
}

pub(crate) fn install_hook(hook: HookName) -> Result<PathBuf> {
    let root = git_hook_repo_root()?;
    let path = genesis_git_hooks::install(&root, hook, PRE_COMMIT_HOOK_MARKER, &hook_script())
        .map_err(|e| anyhow!("{}", hook_error_message(e)))?;
    Ok(path)
}

fn uninstall_hook(hook: HookName) -> Result<PathBuf> {
    let root = git_hook_repo_root()?;
    let path = genesis_git_hooks::uninstall(&root, hook, PRE_COMMIT_HOOK_MARKER)
        .map_err(|e| anyhow!("{}", hook_error_message(e)))?;
    Ok(path)
}

fn hook_error_message(err: genesis_git_hooks::GitHooksError) -> String {
    use genesis_git_hooks::GitHooksError as E;
    match err {
        E::ForeignHook { path, action } => match action {
            "remove" => format!(
                "refusing to remove hook not installed by Pretender: {}",
                path.display()
            ),
            _ => format!(
                "refusing to overwrite hook not installed by Pretender: {}",
                path.display()
            ),
        },
        other => format!("hook operation failed: {other}"),
    }
}

fn git_hook_repo_root() -> Result<PathBuf> {
    genesis_git_hooks::repo_root().map_err(|e| anyhow!("{}", e))
}

fn github_ci_workflow_path() -> PathBuf {
    PathBuf::from(".github/workflows/pretender.yml")
}

pub(crate) fn write_github_ci_workflow() -> Result<()> {
    let path = github_ci_workflow_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create workflow dir: {}", parent.display()))?;
    }
    fs::write(&path, github_ci_workflow_contents())
        .with_context(|| format!("failed to write workflow: {}", path.display()))?;
    Ok(())
}
