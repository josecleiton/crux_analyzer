//! `crux-analyzer` — analyzes a Rust + Crux crate and emits the semantic
//! model as JSON (the contract in `shared/schema/crux-model.schema.json`)
//! or as generated documentation (Markdown / Mermaid).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use crux_analyzer_i18n::Locale;
use crux_analyzer_parser::Limits;

mod i18n;
mod watch;

use i18n::Messages;

/// Stack for the analysis thread.
///
/// `syn::parse_file` recurses over nested expressions and so does the walker, so
/// nesting depth in the *input* becomes stack depth here. The parser's own depth
/// caps bound the walkers, but not `syn` itself, and a stack overflow aborts the
/// process rather than returning an error. A big stack is the standard answer;
/// see `docs/security.md`.
const ANALYSIS_STACK_SIZE: usize = 64 << 20;

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

    /// Exit non-zero if the parser reports any warning. Output is still
    /// written — the failure is the signal, for CI.
    #[arg(long, global = true)]
    deny_warnings: bool,

    #[command(flatten)]
    limits: LimitArgs,
}

/// Caps on analyzing source you may not control.
///
/// The defaults suit any real application; they exist so that a hostile or
/// pathological tree cannot hang the analyzer or exhaust its memory. Hitting one
/// emits an `analysis-truncated`, `file-too-large` or `input-too-large` warning,
/// so `--deny-warnings` turns a truncated run into a failed one.
#[derive(clap::Args)]
struct LimitArgs {
    /// Skip `.rs` files larger than this many bytes.
    #[arg(long, global = true, value_name = "BYTES",
          default_value_t = Limits::DEFAULT_MAX_FILE_SIZE)]
    max_file_size: u64,
    /// Stop reading once this many bytes of source have been loaded.
    #[arg(long, global = true, value_name = "BYTES",
          default_value_t = Limits::DEFAULT_MAX_TOTAL_SIZE)]
    max_total_size: u64,
    /// Expression-walking steps allowed per Core.
    #[arg(long, global = true, value_name = "STEPS",
          default_value_t = Limits::DEFAULT_MAX_STEPS)]
    max_steps: u64,
}

impl LimitArgs {
    fn to_limits(&self) -> Limits {
        Limits {
            max_file_size: self.max_file_size,
            max_total_size: self.max_total_size,
            max_steps: self.max_steps,
            ..Limits::default()
        }
    }
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
    /// Report how much of the analyzed app's states carry a description.
    Coverage {
        #[command(flatten)]
        input: InputArgs,
        /// Exit non-zero when the share of described states is below this
        /// percentage.
        #[arg(long, value_name = "PERCENT")]
        min: Option<u8>,
        /// List the states that have no description.
        #[arg(long)]
        list: bool,
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
    // All analysis happens on a thread with a stack large enough for deeply
    // nested input; the main thread only waits. A panic there is reported as a
    // failure rather than unwinding out of `main`.
    std::thread::Builder::new()
        .stack_size(ANALYSIS_STACK_SIZE)
        .spawn(run)
        .and_then(|handle| handle.join().map_err(|_| std::io::Error::other("panicked")))
        .unwrap_or(ExitCode::FAILURE)
}

fn run() -> ExitCode {
    let cli = Cli::parse();
    let messages = Messages::new(cli.locale.unwrap_or_else(Locale::from_env));
    let deny_warnings = cli.deny_warnings;
    let limits = cli.limits.to_limits();

    // `coverage` reports on the model rather than emitting it, so it has no
    // renderer, no output file and nothing to watch.
    if let Command::Coverage { input, min, list } = cli.command {
        let project_name = input.name.unwrap_or_else(|| directory_name(&input.src));
        return report_coverage(
            &input.src,
            &project_name,
            min,
            list,
            deny_warnings,
            &limits,
            &messages,
        );
    }

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
            Command::Coverage { .. } => unreachable!("handled above"),
        };

    let project_name = input.name.clone().unwrap_or_else(|| directory_name(&input.src));
    let run_analysis = || {
        run_once(
            &input.src,
            &project_name,
            out.as_deref(),
            render,
            deny_warnings,
            &limits,
            &messages,
        )
    };

    if watching {
        // The watcher ignores each run's exit code, so `--deny-warnings`
        // reports without ending the session.
        watch::watch(&input.src, out.as_deref(), &messages, run_analysis)
    } else {
        run_analysis()
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
    deny_warnings: bool,
    limits: &Limits,
    messages: &Messages,
) -> ExitCode {
    let locale = messages.locale();
    let outcome = match crux_analyzer_parser::parse_project_with(src, project_name, limits) {
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

    // Output is written either way: the exit code is the signal, so a pipeline
    // fails while a human still gets the artifact to look at.
    if deny_warnings && !outcome.warnings.is_empty() {
        eprintln!(
            "{}: {}",
            messages.error_prefix(),
            messages.warnings_denied(outcome.warnings.len())
        );
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}

/// Prints the documentation-coverage report and applies `--min`.
fn report_coverage(
    src: &Path,
    project_name: &str,
    min: Option<u8>,
    list: bool,
    deny_warnings: bool,
    limits: &Limits,
    messages: &Messages,
) -> ExitCode {
    let locale = messages.locale();
    let outcome = match crux_analyzer_parser::parse_project_with(src, project_name, limits) {
        Ok(outcome) => outcome,
        Err(err) => {
            eprintln!("{}: {}", messages.error_prefix(), err.message(locale));
            return ExitCode::FAILURE;
        }
    };
    for warning in &outcome.warnings {
        eprintln!("{}: {}", messages.warning_prefix(), warning.render(locale));
    }

    let report = crux_analyzer_docgen::coverage(&outcome.project);
    print!("{}", messages.coverage_report(&report, list));

    let mut failed = false;
    if let Some(min) = min {
        if !report.states.meets(min) {
            eprintln!(
                "{}: {}",
                messages.error_prefix(),
                messages.coverage_below_minimum(report.states.percent(), min)
            );
            failed = true;
        }
    }
    if deny_warnings && !outcome.warnings.is_empty() {
        eprintln!(
            "{}: {}",
            messages.error_prefix(),
            messages.warnings_denied(outcome.warnings.len())
        );
        failed = true;
    }

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn directory_name(src: &Path) -> String {
    src.canonicalize()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_else(|| "project".to_string())
}
