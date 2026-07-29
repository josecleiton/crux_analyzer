//! `crux-analyzer` — analyzes a Rust + Crux crate and emits the semantic
//! model as JSON (the contract in `shared/schema/crux-model.schema.json`).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "crux-analyzer", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Analyze a crate's sources and emit the semantic model as JSON.
    Generate {
        /// Directory containing the Rust sources to analyze (e.g. `shared/src`).
        #[arg(long)]
        src: PathBuf,
        /// Project name in the emitted model (defaults to the src directory name).
        #[arg(long)]
        name: Option<String>,
        /// Output file (defaults to stdout).
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Generate { src, name, out } => generate(&src, name.as_deref(), out.as_deref()),
    }
}

fn generate(
    src: &std::path::Path,
    name: Option<&str>,
    out: Option<&std::path::Path>,
) -> ExitCode {
    let project_name = name
        .map(str::to_string)
        .or_else(|| {
            src.canonicalize()
                .ok()?
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "project".to_string());

    let outcome = match crux_analyzer_parser::parse_project(src, &project_name) {
        Ok(outcome) => outcome,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };

    for warning in &outcome.warnings {
        eprintln!("warning: {warning}");
    }

    let json = match serde_json::to_string_pretty(&outcome.project) {
        Ok(json) => json,
        Err(err) => {
            eprintln!("error: failed to serialize model: {err}");
            return ExitCode::FAILURE;
        }
    };

    match out {
        Some(path) => {
            if let Err(err) = std::fs::write(path, json + "\n") {
                eprintln!("error: failed to write {}: {err}", path.display());
                return ExitCode::FAILURE;
            }
            eprintln!(
                "wrote {} ({} core(s), {} warning(s))",
                path.display(),
                outcome.project.cores.len(),
                outcome.warnings.len()
            );
        }
        None => println!("{json}"),
    }

    ExitCode::SUCCESS
}
