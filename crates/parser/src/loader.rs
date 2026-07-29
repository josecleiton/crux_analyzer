//! Loads and parses every `.rs` file under a source directory.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::ParseError;

pub(crate) struct SourceFile {
    pub path: PathBuf,
    pub ast: syn::File,
}

/// Walks `src_dir` recursively, parsing every `.rs` file (skipping `target/`).
pub(crate) fn load_sources(src_dir: &Path) -> Result<Vec<SourceFile>, ParseError> {
    let mut sources = Vec::new();

    for entry in WalkDir::new(src_dir)
        .into_iter()
        .filter_entry(|e| e.file_name() != "target")
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let content = std::fs::read_to_string(path)
            .map_err(|err| ParseError::Io(path.to_path_buf(), err))?;
        let ast = syn::parse_file(&content)
            .map_err(|err| ParseError::Syntax(path.to_path_buf(), err))?;
        sources.push(SourceFile {
            path: path.to_path_buf(),
            ast,
        });
    }

    Ok(sources)
}

/// Builds an in-memory source set from `(virtual path, code)` pairs — test helper.
#[cfg(test)]
pub(crate) fn sources_from_str(files: &[(&str, &str)]) -> Vec<SourceFile> {
    files
        .iter()
        .map(|(path, code)| SourceFile {
            path: PathBuf::from(path),
            ast: syn::parse_file(code).expect("test fixture must parse"),
        })
        .collect()
}
