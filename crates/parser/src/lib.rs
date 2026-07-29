//! Static parser for Rust + Crux applications.
//!
//! Reads Rust sources, walks the AST via `syn` and identifies Cores, states,
//! events and transitions, emitting a [`Project`]. It never knows about React
//! or any client of the model, and never depends on Crux itself.
//!
//! # How extraction works
//!
//! 1. All `.rs` files under the source directory are parsed and indexed
//!    (enums, functions) — no module-tree resolution, the crate is flattened.
//! 2. Each `impl App for X` block becomes a Core; its `Event` associated type
//!    seeds the set of event enums (following nested event enums).
//! 3. State machines are detected by assignment analysis: an enum assigned to
//!    a model field (`*.state = Enum::Variant`) that is also matched against
//!    is a state machine — naming conventions are not required.
//! 4. Transitions are extracted by walking `update` and every helper it calls
//!    (cross-file), carrying the current event label and the source-state set
//!    from `matches!` guards and `match`-on-state arms (wildcards resolve to
//!    the complement of the variants matched earlier).
//!
//! Transitions whose event or source state cannot be inferred statically are
//! dropped and reported as [`Warning`]s. Known future work:
//! predicate-method guards (e.g. `self.state.is_active()`), struct resets via
//! `Default::default()` implying the `#[default]` variant, and a schema
//! representation for "from any state" transitions.

use std::path::{Path, PathBuf};

use crux_analyzer_i18n::Locale;
use crux_analyzer_model::Project;

pub use limits::Limits;

mod annotations;
mod ast_util;
mod core_finder;
mod emit;
mod i18n;
mod index;
mod limits;
mod loader;
mod state_enum;
#[cfg(test)]
mod tests;
mod transitions;

/// Analysis errors.
#[derive(Debug)]
pub enum ParseError {
    /// A source file could not be read.
    Io(PathBuf, std::io::Error),
    /// A source file is not valid Rust.
    Syntax(PathBuf, syn::Error),
    /// No `impl App for ...` block was found in the sources.
    NoCoreFound,
}

/// Renders in English, the source locale. Use [`ParseError::message`] to
/// render in a caller-chosen locale.
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message(Locale::En))
    }
}

impl std::error::Error for ParseError {}

/// What the parser could not infer, as data.
///
/// The prose lives in [`crate::i18n`], keyed off these variants, so a
/// diagnostic can be rendered in any locale long after it was produced. The
/// interpolated names are identifiers from the analyzed source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarningKind {
    /// An `impl App` block without an `update` fn.
    NoUpdateMethod { core: String },
    /// The assigned value has no payload typing and no resolvable constraints.
    DynamicTarget { machine: String },
    /// A state assignment was reached with no event label in scope.
    UnknownEvent { to: String },
    /// The guard references the state but defeats static analysis.
    UnresolvableSource { to: String },
    /// A doc comment line looked like an annotation but is not one — a typo
    /// (`@failur`), a marker given an argument, or a `@tag` with no usable
    /// name. Reported rather than left inert, so the mistake is visible.
    UnknownAnnotation { annotation: String },
    /// A resource limit stopped the walk before the Core was fully explored,
    /// so the model may be missing transitions. See [`Limits`].
    AnalysisTruncated { core: String, limit: String },
    /// A source file exceeded `--max-file-size` and was skipped.
    FileTooLarge { size: u64, max: u64 },
    /// The run reached `--max-total-size`; the remaining files were skipped.
    InputTooLarge { max: u64 },
    /// A path under the source directory could not be read or walked. Reported
    /// rather than skipped silently: a permission error otherwise yields a
    /// quietly partial model.
    SourceUnreadable { reason: String },
    /// A path was skipped because it is not a regular file — a symlink, a
    /// device, a socket or a FIFO. Following it would read outside the source
    /// tree, or never terminate.
    NotARegularFile,
    /// A file nests brackets deeper than `max` and was skipped without being
    /// parsed: `syn` recurses over nesting, and its stack overflow would abort
    /// the process rather than return an error.
    NestingTooDeep { max: usize },
}

impl WarningKind {
    /// Stable, locale-independent identifier for this diagnostic.
    ///
    /// This — not the prose — is what documentation and tooling should key on.
    pub fn code(&self) -> &'static str {
        match self {
            WarningKind::NoUpdateMethod { .. } => "no-update-method",
            WarningKind::DynamicTarget { .. } => "dynamic-target",
            WarningKind::UnknownEvent { .. } => "unknown-event",
            WarningKind::UnresolvableSource { .. } => "unresolvable-source",
            WarningKind::UnknownAnnotation { .. } => "unknown-annotation",
            WarningKind::AnalysisTruncated { .. } => "analysis-truncated",
            WarningKind::FileTooLarge { .. } => "file-too-large",
            WarningKind::InputTooLarge { .. } => "input-too-large",
            WarningKind::SourceUnreadable { .. } => "source-unreadable",
            WarningKind::NotARegularFile => "not-a-regular-file",
            WarningKind::NestingTooDeep { .. } => "nesting-too-deep",
        }
    }
}

/// A transition (or pattern) the parser saw but could not fully infer.
#[derive(Debug, Clone)]
pub struct Warning {
    pub file: PathBuf,
    pub line: usize,
    pub kind: WarningKind,
}

impl Warning {
    /// `file:line: message`, with the message in `locale`.
    ///
    /// The path is sanitized like the message: it comes from the analyzed tree,
    /// and this string is written to a terminal.
    pub fn render(&self, locale: Locale) -> String {
        format!(
            "{}:{}: {}",
            i18n::sanitize(&self.file.display().to_string()),
            self.line,
            self.kind.message(locale)
        )
    }
}

/// Renders in English, the source locale. Use [`Warning::render`] to render in
/// a caller-chosen locale.
impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.render(Locale::En))
    }
}

/// Result of analyzing a crate: the semantic model plus diagnostics.
#[derive(Debug)]
pub struct ParseOutcome {
    pub project: Project,
    pub warnings: Vec<Warning>,
}

/// Parses the Rust sources under `src_dir` with the default [`Limits`].
pub fn parse_project(src_dir: &Path, project_name: &str) -> Result<ParseOutcome, ParseError> {
    parse_project_with(src_dir, project_name, &Limits::default())
}

/// Parses the Rust sources under `src_dir` under caller-chosen [`Limits`].
///
/// Analyzing source you do not control is the case these limits exist for; the
/// defaults are sized for that, and a limit that fires is reported as a
/// [`Warning`] rather than silently truncating the model.
pub fn parse_project_with(
    src_dir: &Path,
    project_name: &str,
    limits: &Limits,
) -> Result<ParseOutcome, ParseError> {
    let mut warnings = Vec::new();
    let sources = loader::load_sources(src_dir, limits, &mut warnings)?;
    parse_loaded(&sources, project_name, limits, warnings)
}

/// Same as [`parse_project`] over already-loaded sources (used by tests).
#[cfg(test)]
pub(crate) fn parse_sources(
    sources: &[loader::SourceFile],
    project_name: &str,
) -> Result<ParseOutcome, ParseError> {
    parse_loaded(sources, project_name, &Limits::default(), Vec::new())
}

fn parse_loaded(
    sources: &[loader::SourceFile],
    project_name: &str,
    limits: &Limits,
    loader_warnings: Vec<Warning>,
) -> Result<ParseOutcome, ParseError> {
    let index = index::build_index(sources);
    let detection = state_enum::find_state_machines(&index);
    let machines = detection.machines;
    // State enums are excluded from the event closure (carried as an event
    // payload they are data, not nested event enums), and only enums the
    // code dispatches on qualify as nested event enums at all.
    let machine_enums: std::collections::BTreeSet<String> =
        machines.iter().map(|m| m.enum_name.clone()).collect();
    let cores = core_finder::find_cores(&index, &machine_enums, &detection.dispatched_enums);
    if cores.is_empty() {
        return Err(ParseError::NoCoreFound);
    }
    let mut warnings = loader_warnings;
    warnings.extend(annotation_warnings(&machines));
    let mut model_cores = Vec::new();

    for core in &cores {
        let extraction = transitions::extract(&index, core, &machines, limits, &mut warnings);
        model_cores.push(emit::to_core(core, &machines, extraction));
    }

    Ok(ParseOutcome {
        project: Project {
            project: project_name.to_string(),
            cores: model_cores,
        },
        warnings,
    })
}

/// Annotation-shaped lines the grammar did not recognize, as warnings.
///
/// Only enums that became state machines are inspected, so a doc comment on an
/// unrelated enum can never produce noise. Deduplicated because
/// `use X as Y` registers a clone of the same declaration under a second name,
/// which makes one typo reachable twice — through a set, so a file full of
/// annotation typos costs linear time rather than quadratic.
fn annotation_warnings(machines: &[state_enum::StateMachine]) -> Vec<Warning> {
    let mut warnings: Vec<Warning> = Vec::new();
    let mut seen: std::collections::HashSet<(PathBuf, usize, String)> =
        std::collections::HashSet::new();
    for machine in machines {
        let blocks = std::iter::once(&machine.docs).chain(&machine.variant_docs);
        for problem in blocks.flat_map(|block| &block.problems) {
            if !seen.insert((machine.file.clone(), problem.line, problem.text.clone())) {
                continue;
            }
            warnings.push(Warning {
                file: machine.file.clone(),
                line: problem.line,
                kind: WarningKind::UnknownAnnotation {
                    annotation: problem.text.clone(),
                },
            });
        }
    }
    warnings
}
