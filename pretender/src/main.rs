mod metrics;
mod model;
mod python;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use crate::model::Metric;
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
        
        let metric = metrics::CyclomaticComplexity;
        let mut results: Vec<(String, u32)> = module
            .units
            .iter()
            .map(|u| (u.name.clone(), metric.calculate(u)))
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

    // In a more complex app, this registry would be passed around or be a global.
    // For now, we'll keep the logic here but use the Registry pattern to structure it.
    let mut registry = model::ParserRegistry::new();
    registry.register(Box::new(python::PythonParser));

    registry
        .get_for_extension(ext)
        .map(|_p| {
            // This is a bit awkward because of trait object cloning/ownership,
            // but it demonstrates the registry pattern.
            // In a real refactor, we'd probably store Arc<dyn Parser>.
            // For now, let's just re-instantiate or fix the ownership.
            match ext {
                "py" => Box::new(python::PythonParser) as Box<dyn model::Parser>,
                _ => unreachable!(),
            }
        })
        .ok_or_else(|| anyhow!("unsupported language for extension: .{}", ext))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.command.run()
}
