//! Static parser for Rust + Crux applications.
//!
//! Future responsibility: read Rust files, walk the AST via `syn` and
//! identify Core, State, Event, Effect and transitions, emitting a
//! [`Project`]. It never knows about React or any client of the model.
//!
//! In the MVP this crate is a stub: it only pins the public signature.
//! The `syn` dependency lands together with the real implementation.

use std::path::Path;

use crux_analyzer_model::Project;

/// Analysis errors.
#[derive(Debug)]
pub enum ParseError {
    /// The parser is not implemented yet (the MVP UI reads fake JSON).
    Unimplemented,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Unimplemented => {
                write!(f, "the parser is not implemented yet")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Parses the Rust files in `paths` and produces the project's semantic model.
pub fn parse_project(paths: &[&Path]) -> Result<Project, ParseError> {
    let _ = paths;
    Err(ParseError::Unimplemented)
}
