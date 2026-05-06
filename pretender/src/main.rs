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
    Complexity(ComplexityArgs),
}

#[derive(Parser)]
struct ComplexityArgs {
    path: PathBuf,
}

trait Executable {
    fn run(&self) -> Result<()>;
}

impl Executable for Commands {
    fn run(&self) -> Result<()> {
        match self {
            Commands::Complexity(args) => args.run(),
        }
    }
}

impl Executable for ComplexityArgs {
    fn run(&self) -> Result<()> {
        let source = std::fs::read_to_string(&self.path)?;
        let parser = get_parser(&self.path)?;
        let module = parser.parse(&self.path, &source)?;
        let mut results: Vec<(String, u32)> = module
            .units
            .iter()
            .map(|u| (u.name.clone(), metrics::cyclomatic(u)))
            .collect();
        results.sort_by_key(|(_, score)| Reverse(*score));
        for (name, score) in &results {
            println!("{name}: {score}");
        }
        Ok(())
    }
}

fn get_parser(path: &Path) -> Result<Box<dyn model::Parser>> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("no file extension for {}", path.display()))?;

    let parsers: Vec<Box<dyn model::Parser>> = vec![
        Box::new(python::PythonParser),
    ];

    for parser in parsers {
        if parser.extensions().contains(&ext) {
            return Ok(parser);
        }
    }

    Err(anyhow!("unsupported language for extension: .{}", ext))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.command.run()
}
