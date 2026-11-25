//! Command runners for the analysis, report, mutation, explain, doctor,
//! and feedback subcommands.
//!
//! Each `Executable::run` implementation orchestrates loading config and
//! inputs, invoking the corresponding engine, and printing the result.
//!
//! # Responsibilities
//! Submodule wiring for the command runners: complexity and duplication
//! runners plus their printers in `complexity`/`duplication`, the
//! mutation runner plus its printers in `mutation`, and the feedback
//! runner plus repository extraction in `feedback`. The explain and
//! doctor runners and the doctor format resolution stay here.
//!
//! # Rationale
//! Extracted from `main.rs` (pretender-5zo.2.5); the oversized runners
//! were split into verbatim-behaving stage helpers (diagnostics/table
//! printing, source collection, group printing, result emission) so every
//! function stays within the structural limits with output unchanged.
//! Split into per-command siblings (pretender-5zo.15.14) so every file
//! stays under the file-size gate.
mod complexity;
mod duplication;
mod feedback;
mod mutation;

use crate::cli::args::{DoctorArgs, ExplainArgs};
use crate::cli::Executable;
use anyhow::Result;
use pretender::{doctor, explain};
use std::process::ExitCode;

impl Executable for ExplainArgs {
    fn run(&self) -> Result<ExitCode> {
        explain::run(&self.metric)?;
        Ok(ExitCode::SUCCESS)
    }
}

impl Executable for DoctorArgs {
    fn run(&self) -> Result<ExitCode> {
        // Doctor format from global CLI flags (--json/--human or TTY auto-detect)
        let format = resolve_doctor_format();
        doctor::run_doctor(format)
    }
}

/// Resolve the doctor output format from global CLI args or TTY auto-detect.
fn resolve_doctor_format() -> genesis::guide::OutputFormat {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--json" || a == "-j") {
        genesis::guide::OutputFormat::Json
    } else {
        genesis::guide::OutputFormat::Human
    }
}

#[cfg(test)]
mod managed_block_ownership_tests {
    /// AGENTS.md regions pretender owns.
    ///
    /// Ownership rule: only the tool that publishes a region may write it.
    /// pretender publishes none — WAI belongs to wai and DONT to dont — so
    /// `pretender init` injects no blocks and the doctor registers no
    /// block-drift target. A future entry here is a deliberate, test-visible
    /// decision that must name the publishing tool first.
    fn owned_agents_regions() -> Vec<&'static str> {
        vec![]
    }

    #[test]
    fn supported_managed_block_set_is_empty() {
        let owned = owned_agents_regions();
        assert!(
            owned.is_empty(),
            "ownership rule violated: only a tool that publishes an AGENTS.md \
             region may write it, but pretender claims {owned:?} — WAI and DONT \
             are authored by wai and dont"
        );
    }
}
