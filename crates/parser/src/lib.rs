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

use crux_analyzer_model::Project;

mod ast_util;
mod core_finder;
mod emit;
mod index;
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

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Io(path, err) => write!(f, "failed to read {}: {err}", path.display()),
            ParseError::Syntax(path, err) => {
                write!(f, "failed to parse {}: {err}", path.display())
            }
            ParseError::NoCoreFound => write!(f, "no `impl App for ...` block found"),
        }
    }
}

impl std::error::Error for ParseError {}

/// A transition (or pattern) the parser saw but could not fully infer.
#[derive(Debug, Clone)]
pub struct Warning {
    pub file: PathBuf,
    pub line: usize,
    pub message: String,
}

impl std::fmt::Display for Warning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.file.display(), self.line, self.message)
    }
}

/// Result of analyzing a crate: the semantic model plus diagnostics.
#[derive(Debug)]
pub struct ParseOutcome {
    pub project: Project,
    pub warnings: Vec<Warning>,
}

/// Parses the Rust sources under `src_dir` and produces the semantic model.
pub fn parse_project(src_dir: &Path, project_name: &str) -> Result<ParseOutcome, ParseError> {
    let sources = loader::load_sources(src_dir)?;
    parse_sources(&sources, project_name)
}

/// Same as [`parse_project`] over already-loaded sources (used by tests).
pub(crate) fn parse_sources(
    sources: &[loader::SourceFile],
    project_name: &str,
) -> Result<ParseOutcome, ParseError> {
    let index = index::build_index(sources);
    let machines = state_enum::find_state_machines(&index);
    // State enums are excluded from the event/effect closures: carried as an
    // event payload they are data, not nested event enums.
    let machine_enums: std::collections::BTreeSet<String> =
        machines.iter().map(|m| m.enum_name.clone()).collect();
    let cores = core_finder::find_cores(&index, &machine_enums);
    if cores.is_empty() {
        return Err(ParseError::NoCoreFound);
    }
    let mut warnings = Vec::new();
    let mut model_cores = Vec::new();

    for core in &cores {
        let extraction = transitions::extract(&index, core, &machines, &mut warnings);
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
