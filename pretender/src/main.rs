mod metrics;
mod model;
mod python;

use anyhow::{anyhow, Context, Result};
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
        let source = std::fs::read_to_string(&self.path)
            .with_context(|| format!("failed to read source file: {}", self.path.display()))?;
        let parser = get_parser(&self.path)?;
        let (module, diagnostics) = parser.parse(&self.path, &source)?;
        
        if !diagnostics.is_empty() {
            for diag in &diagnostics {
                eprintln!("{:?}: {}", diag.severity, diag.message);
                if let Some(span) = &diag.span {
                    eprintln!("  at lines {}-{}", span.start_line, span.end_line);
                }
            }
            eprintln!(); // Blank line for separation
        }

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
        .ok_or_else(|| anyhow!("missing file extension for path: {}", path.display()))?;

    let mut registry = model::ParserRegistry::new();
    registry.register(Box::new(python::PythonParser));

    registry
        .get_for_extension(ext)
        .map(|_p| {
            match ext {
                "py" => Box::new(python::PythonParser) as Box<dyn model::Parser>,
                _ => unreachable!("registry returned parser for unsupported extension .{}", ext),
            }
        })
        .ok_or_else(|| anyhow!("unsupported file extension '.{}' for complexity analysis", ext))
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.command.run()
}
