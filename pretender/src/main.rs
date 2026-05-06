mod metrics;
mod model;
mod python;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use std::cmp::Reverse;
use std::path::{Path, PathBuf};

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

fn get_parser(path: &Path) -> Result<Box<dyn model::Parser>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("no file extension for {}", path.display()))?;

    match ext {
        "py" => Ok(Box::new(python::PythonParser)),
        _ => Err(anyhow!("unsupported language for extension: .{}", ext)),
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Complexity { path } => {
            let source = std::fs::read_to_string(&path)?;
            let parser = get_parser(&path)?;
            let module = parser.parse(&path, &source)?;
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
