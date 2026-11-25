//! `pretender doctor` — health checks over the current working repository.
//!
//! Responsibilities: run the `DoctorRunner` over the `DoctorCheck`
//! implementations in `checks` and render the resulting report as human
//! text or JSON.
//! Rationale: each check is a small self-contained type so it can pass,
//! warn, or fail independently; rendering is kept separate from checking.

mod checks;
mod render;
mod steps;

use anyhow::Result;
use genesis::doctor::{DoctorReport, DoctorRunner};
use genesis::guide::OutputFormat;
use std::process::ExitCode;

/// Marker line identifying a pre-commit hook as Pretender-managed.
const PRE_COMMIT_HOOK_MARKER: &str = "# Installed by Pretender.";

pub fn run_doctor(format: OutputFormat) -> Result<ExitCode> {
    let report = run_checks()?;
    render::render(&report, format)?;
    Ok(render::exit_code(&report))
}

/// Build the genesis runner over every doctor check and execute it in the
/// current working directory.
fn run_checks() -> Result<DoctorReport> {
    let runner = DoctorRunner::new(vec![
        Box::new(checks::GitContextCheck),
        Box::new(checks::ConfigPresentCheck),
        Box::new(checks::ConfigValidCheck),
        Box::new(checks::HookInstalledCheck),
        Box::new(checks::HookExecutableCheck),
        Box::new(checks::HookLocationCheck),
        Box::new(checks::HooksConfigMismatchCheck),
        Box::new(checks::PluginManifestsCheck),
    ])
    .with_tool_name("pretender");

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    runner
        .run(&cwd, false)
        .map_err(|e| anyhow::anyhow!("{}", e))
}
