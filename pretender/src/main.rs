mod metrics;
mod model;
mod python;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::cmp::Reverse;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "pretender")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Show cyclomatic complexity for each function, sorted worst-first
    Complexity { path: PathBuf },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Complexity { path } => {
            let source = std::fs::read_to_string(&path)?;
            let module = python::parse(&path, &source)?;
            let mut results: Vec<(String, u32)> = module
                .units
                .iter()
                .map(|u| (u.name.clone(), metrics::cyclomatic(u)))
                .collect();
            results.sort_by_key(|(_, score)| Reverse(*score));
            for (name, score) in &results {
                println!("{name}: {score}");
            }
        }
    }
    Ok(())
}
