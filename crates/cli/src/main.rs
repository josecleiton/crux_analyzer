//! `crux-analyzer` — analyzes a Rust + Crux crate and emits the semantic
//! model as JSON (the contract in `shared/schema/crux-model.schema.json`)
//! or as generated documentation (Markdown / Mermaid).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

mod watch;

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
        #[command(flatten)]
        input: InputArgs,
        /// Output file (defaults to stdout).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Keep watching the sources and regenerate on change.
        #[arg(long)]
        watch: bool,
    },
    /// Analyze a crate's sources and emit documentation.
    Docs {
        #[command(flatten)]
        input: InputArgs,
        /// Output format.
        #[arg(long, value_enum, default_value_t = DocsFormat::Markdown)]
        format: DocsFormat,
        /// Output file (defaults to stdout).
        #[arg(long)]
        out: Option<PathBuf>,
        /// Keep watching the sources and regenerate on change.
        #[arg(long)]
        watch: bool,
    },
}

#[derive(clap::Args)]
struct InputArgs {
    /// Directory containing the Rust sources to analyze (e.g. `shared/src`).
    #[arg(long)]
    src: PathBuf,
    /// Project name in the emitted model (defaults to the src directory name).
    #[arg(long)]
    name: Option<String>,
}

#[derive(Clone, Copy, ValueEnum)]
enum DocsFormat {
    Markdown,
    Mermaid,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let (input, out, watching, render): (InputArgs, Option<PathBuf>, bool, Renderer) =
        match cli.command {
            Command::Generate { input, out, watch } => (input, out, watch, render_json),
            Command::Docs {
                input,
                format,
                out,
                watch,
            } => (
                input,
                out,
                watch,
                match format {
                    DocsFormat::Markdown => render_markdown,
                    DocsFormat::Mermaid => render_mermaid,
                },
            ),
        };

    let project_name = input.name.clone().unwrap_or_else(|| directory_name(&input.src));
    let run = || run_once(&input.src, &project_name, out.as_deref(), render);

    if watching {
        watch::watch(&input.src, run)
    } else {
        run()
    }
}

type Renderer = fn(&crux_analyzer_model::Project) -> String;

fn render_json(project: &crux_analyzer_model::Project) -> String {
    serde_json::to_string_pretty(project).expect("model serialization cannot fail") + "\n"
}

fn render_markdown(project: &crux_analyzer_model::Project) -> String {
    crux_analyzer_docgen::markdown(project)
}

fn render_mermaid(project: &crux_analyzer_model::Project) -> String {
    // Multiple diagrams are separated by comment headers so the output can
    // be split per machine.
    crux_analyzer_docgen::mermaid_diagrams(project)
        .into_iter()
        .map(|d| format!("%% {} / {}\n{}\n", d.core, d.machine, d.mermaid))
        .collect::<Vec<_>>()
        .join("\n")
}

fn run_once(
    src: &Path,
    project_name: &str,
    out: Option<&Path>,
    render: Renderer,
) -> ExitCode {
    let outcome = match crux_analyzer_parser::parse_project(src, project_name) {
        Ok(outcome) => outcome,
        Err(err) => {
            eprintln!("error: {err}");
            return ExitCode::FAILURE;
        }
    };

    for warning in &outcome.warnings {
        eprintln!("warning: {warning}");
    }

    let rendered = render(&outcome.project);
    match out {
        Some(path) => {
            if let Err(err) = std::fs::write(path, rendered) {
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
        None => print!("{rendered}"),
    }

    ExitCode::SUCCESS
}

fn directory_name(src: &Path) -> String {
    src.canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "project".to_string())
}
