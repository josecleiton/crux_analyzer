//! `crux-analyzer` — analyzes a Rust + Crux crate and emits the semantic
//! model as JSON (the contract in `shared/schema/crux-model.schema.json`)
//! or as generated documentation (Markdown / Mermaid).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use crux_analyzer_i18n::Locale;

mod i18n;
mod watch;

use i18n::Messages;

#[derive(Parser)]
#[command(name = "crux-analyzer", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Language of the generated prose and the CLI's own output: `en` or
    /// `pt-BR`. Defaults to CRUX_ANALYZER_LOCALE, then LC_ALL / LC_MESSAGES /
    /// LANG, then `en`. Identifiers from the analyzed code are never
    /// translated.
    #[arg(long, global = true, value_name = "LOCALE")]
    locale: Option<Locale>,
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
    let messages = Messages::new(cli.locale.unwrap_or_else(Locale::from_env));
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
    let run = || run_once(&input.src, &project_name, out.as_deref(), render, &messages);

    if watching {
        watch::watch(&input.src, &messages, run)
    } else {
        run()
    }
}

type Renderer = fn(&crux_analyzer_model::Project, Locale) -> String;

fn render_json(project: &crux_analyzer_model::Project, _locale: Locale) -> String {
    // The model contract is locale-independent: it carries only identifiers
    // read out of the analyzed source, so no locale reaches the JSON.
    serde_json::to_string_pretty(project).expect("model serialization cannot fail") + "\n"
}

fn render_markdown(project: &crux_analyzer_model::Project, locale: Locale) -> String {
    crux_analyzer_docgen::markdown(project, locale)
}

fn render_mermaid(project: &crux_analyzer_model::Project, locale: Locale) -> String {
    // Multiple diagrams are separated by comment headers so the output can
    // be split per machine.
    crux_analyzer_docgen::mermaid_diagrams(project, locale)
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
    messages: &Messages,
) -> ExitCode {
    let locale = messages.locale();
    let outcome = match crux_analyzer_parser::parse_project(src, project_name) {
        Ok(outcome) => outcome,
        Err(err) => {
            eprintln!("{}: {}", messages.error_prefix(), err.message(locale));
            return ExitCode::FAILURE;
        }
    };

    for warning in &outcome.warnings {
        eprintln!("{}: {}", messages.warning_prefix(), warning.render(locale));
    }

    let rendered = render(&outcome.project, locale);
    match out {
        Some(path) => {
            if let Err(err) = std::fs::write(path, rendered) {
                eprintln!(
                    "{}: {}: {err}",
                    messages.error_prefix(),
                    messages.failed_to_write(path)
                );
                return ExitCode::FAILURE;
            }
            eprintln!(
                "{}",
                messages.wrote_summary(
                    path,
                    outcome.project.cores.len(),
                    outcome.warnings.len()
                )
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
