//! Runner for the `duplication` subcommand: clone detection and report.
//!
//! # Responsibilities
//! `DuplicationArgs::run`, the duplicable-source reader with verbatim
//! skip/warn behavior, and the per-group printer.
//!
//! # Rationale
//! Split out of `cli/runs.rs` (pretender-5zo.15.14) along the
//! duplication seam so every file stays under the file-size gate;
//! bodies moved verbatim with output unchanged.
use crate::check::collect_input_files;
use crate::cli::args::DuplicationArgs;
use crate::cli::infra::{load_config, warn_if_no_config};
use crate::cli::Executable;
use anyhow::Result;
use pretender::duplication;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

impl Executable for DuplicationArgs {
    fn run(&self) -> Result<ExitCode> {
        warn_if_no_config();
        let config = load_config()?;
        let file_paths = collect_input_files(&self.paths, &config)?;
        let min_nodes = self.min_nodes.unwrap_or(10) as usize;
        let files = collect_duplication_sources(&file_paths);

        let groups = duplication::detect_clones(&files, min_nodes, self.cross_file)?;

        if groups.is_empty() {
            println!("No structural clones detected.");
            return Ok(ExitCode::SUCCESS);
        }

        println!("Found {} clone group(s):\n", groups.len());
        print_clone_groups(&groups);

        Ok(ExitCode::SUCCESS)
    }
}

/// Explain a zero-supported-file scan instead of silently reporting "clean".
fn warn_no_supported_sources() {
    eprintln!(
        "warning: no supported source files found to scan for clones (supported extensions: rs, py, js, jsx, mjs, cjs, ts, mts, tsx, cts, rb, go, java, c, h, cpp, cc, cxx, hpp, hxx, jl, r, R)"
    );
}

/// Read the duplicable sources, skipping paths without a supported language
/// and warning on unreadable files (verbatim skip/warn behavior).
fn collect_duplication_sources(file_paths: &[PathBuf]) -> Vec<(PathBuf, String)> {
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for path in file_paths {
        if duplication::ts_language_for_path(path).is_none() {
            continue;
        }
        match fs::read_to_string(path) {
            Ok(source) => files.push((path.clone(), source)),
            Err(e) => eprintln!("warning: skipping {}: {e}", path.display()),
        }
    }
    if files.is_empty() {
        warn_no_supported_sources();
    }
    files
}

/// Print each clone group with node count, similarity, and locations.
fn print_clone_groups(groups: &[duplication::CloneGroup]) {
    for (i, group) in groups.iter().enumerate() {
        println!(
            "Clone {} ({} nodes, similarity: {}%):",
            i + 1,
            group.node_count,
            group.similarity
        );
        for loc in &group.locations {
            println!(
                "  {}:{}-{}",
                loc.file.display(),
                loc.start_line,
                loc.end_line
            );
        }
        println!();
    }
}
