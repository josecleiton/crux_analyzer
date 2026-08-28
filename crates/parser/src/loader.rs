//! Loads and parses every `.rs` file under a source directory.
//!
//! The source tree is untrusted input (see `docs/security.md`), so this is the
//! boundary where three things are enforced:
//!
//! - **only regular files are read.** `walkdir` does not descend symlinked
//!   *directories*, but a symlinked *file* would otherwise be followed —
//!   pulling in source from outside the tree, hanging forever on a FIFO, or
//!   reading `/dev/zero` until memory runs out.
//! - **size is capped**, per file and in total. Every file's `syn` AST is held
//!   for the whole run and an AST is much larger than its source, so unbounded
//!   input is unbounded memory.
//! - **skipped paths are reported.** Dropping walk errors silently would leave
//!   a permission problem looking like a clean, complete model.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::{Limits, ParseError, Warning, WarningKind};

pub(crate) struct SourceFile {
    pub path: PathBuf,
    pub ast: syn::File,
}

/// Walks `src_dir` recursively, parsing every `.rs` file (skipping `target/`).
pub(crate) fn load_sources(
    src_dir: &Path,
    limits: &Limits,
    warnings: &mut Vec<Warning>,
) -> Result<Vec<SourceFile>, ParseError> {
    let mut sources = Vec::new();
    let mut total_bytes: u64 = 0;

    // Sorted, because the order files arrive in is the order declarations land
    // in the index, and the index's tie-breaks are positional. Unsorted, the
    // walk hands back whatever `readdir` returns — stable on one filesystem and
    // different on another — so the same crate analysed on two machines gave two
    // different models, each reproducible on its own machine and neither
    // reproducible on the other.
    //
    // This is necessary and not sufficient: it makes the answer the same
    // everywhere, and `resolve_*` below is what makes it the right one.
    for entry in WalkDir::new(src_dir)
        .follow_links(false)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| e.file_name() != "target")
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                warnings.push(walk_warning(err));
                continue;
            }
        };

        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        // `file_type` describes the entry itself (links are not followed), so a
        // symlinked `.rs` is a symlink here, not a file, and is skipped.
        if !entry.file_type().is_file() {
            warnings.push(Warning {
                file: path.to_path_buf(),
                line: 0,
                kind: WarningKind::NotARegularFile,
            });
            continue;
        }

        let size = match entry.metadata() {
            Ok(metadata) => metadata.len(),
            Err(err) => {
                warnings.push(Warning {
                    file: path.to_path_buf(),
                    line: 0,
                    kind: WarningKind::SourceUnreadable {
                        reason: err.to_string(),
                    },
                });
                continue;
            }
        };
        if size > limits.max_file_size {
            warnings.push(Warning {
                file: path.to_path_buf(),
                line: 0,
                kind: WarningKind::FileTooLarge {
                    size,
                    max: limits.max_file_size,
                },
            });
            continue;
        }
        // Checked before reading, so the cap bounds memory rather than
        // observing that it was already exceeded.
        if total_bytes.saturating_add(size) > limits.max_total_size {
            warnings.push(Warning {
                file: path.to_path_buf(),
                line: 0,
                kind: WarningKind::InputTooLarge {
                    max: limits.max_total_size,
                },
            });
            break;
        }
        total_bytes += size;

        let content =
            std::fs::read_to_string(path).map_err(|err| ParseError::Io(path.to_path_buf(), err))?;
        // Before `syn`, not after: it recurses over nesting, and its stack
        // overflow would abort the process instead of returning an error.
        if nesting_exceeds(&content, limits.max_nesting) {
            warnings.push(Warning {
                file: path.to_path_buf(),
                line: 0,
                kind: WarningKind::NestingTooDeep {
                    max: limits.max_nesting,
                },
            });
            continue;
        }
        let ast =
            syn::parse_file(&content).map_err(|err| ParseError::Syntax(path.to_path_buf(), err))?;
        sources.push(SourceFile {
            path: path.to_path_buf(),
            ast,
        });
    }

    Ok(sources)
}

/// Whether `source` nests brackets deeper than `max`.
///
/// This runs *before* `syn::parse_file`, which recurses over nesting: 5,000
/// nested parens overflow the stack and abort the process, and an abort cannot
/// be caught, retried or reported. A cheap scan of the raw text is the only
/// check that can happen early enough to prevent it.
///
/// Strings, chars and comments are skipped so that a `"((((("` in a literal is
/// not mistaken for real nesting. It only needs to be accurate near the cap —
/// it answers "deeper than `max`?", not "how deep?".
fn nesting_exceeds(source: &str, max: usize) -> bool {
    let bytes = source.as_bytes();
    let mut i = 0;
    let mut depth = 0usize;

    while i < bytes.len() {
        match bytes[i] {
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                i += bytes[i..]
                    .iter()
                    .position(|&b| b == b'\n')
                    .unwrap_or(bytes.len() - i);
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                // Block comments nest in Rust.
                let mut comments = 1;
                i += 2;
                while i < bytes.len() && comments > 0 {
                    if bytes[i] == b'/' && bytes.get(i + 1) == Some(&b'*') {
                        comments += 1;
                        i += 2;
                    } else if bytes[i] == b'*' && bytes.get(i + 1) == Some(&b'/') {
                        comments -= 1;
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            // Raw strings: r"...", r#"..."#, br##"..."##. The hash count that
            // opens one is the hash count that closes it.
            b'r' | b'b' if raw_string_start(bytes, i).is_some() => {
                let (body_start, hashes) = raw_string_start(bytes, i).unwrap();
                i = skip_raw_string(bytes, body_start, hashes);
            }
            b'"' => i = skip_quoted(bytes, i + 1, b'"'),
            // A lifetime (`'a`) is not a char literal; a char literal has a
            // closing quote within a few bytes.
            b'\'' if is_char_literal(bytes, i) => i = skip_quoted(bytes, i + 1, b'\''),
            b'(' | b'[' | b'{' => {
                depth += 1;
                if depth > max {
                    return true;
                }
                i += 1;
            }
            b')' | b']' | b'}' => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            _ => i += 1,
        }
    }
    false
}

/// `Some((index just past the opening quote, hash count))` if a raw string
/// starts at `i`.
fn raw_string_start(bytes: &[u8], i: usize) -> Option<(usize, usize)> {
    let mut j = i;
    if bytes.get(j) == Some(&b'b') {
        j += 1;
    }
    if bytes.get(j) != Some(&b'r') {
        return None;
    }
    j += 1;
    let hashes = bytes[j..].iter().take_while(|&&b| b == b'#').count();
    j += hashes;
    (bytes.get(j) == Some(&b'"')).then_some((j + 1, hashes))
}

fn skip_raw_string(bytes: &[u8], mut i: usize, hashes: usize) -> usize {
    while i < bytes.len() {
        if bytes[i] == b'"' && bytes[i + 1..].iter().take_while(|&&b| b == b'#').count() >= hashes {
            return i + 1 + hashes;
        }
        i += 1;
    }
    bytes.len()
}

/// Skips a `"`- or `'`-delimited literal, honoring backslash escapes.
fn skip_quoted(bytes: &[u8], mut i: usize, terminator: u8) -> usize {
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b if b == terminator => return i + 1,
            _ => i += 1,
        }
    }
    bytes.len()
}

/// Distinguishes `'x'` and `'\n'` from the lifetime in `&'a str`.
fn is_char_literal(bytes: &[u8], i: usize) -> bool {
    match bytes.get(i + 1) {
        Some(b'\\') => true,
        Some(_) => bytes.get(i + 2) == Some(&b'\''),
        None => false,
    }
}

/// A walk error as a warning. `walkdir` reports the path when it has one; a
/// loop or a root failure may not.
fn walk_warning(err: walkdir::Error) -> Warning {
    let file = err.path().map(Path::to_path_buf).unwrap_or_default();
    Warning {
        file,
        line: 0,
        kind: WarningKind::SourceUnreadable {
            reason: err.to_string(),
        },
    }
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

#[cfg(test)]
mod tests {
    use super::nesting_exceeds;

    /// A false positive here would reject legitimate source, so the literal and
    /// comment cases matter as much as the hostile one.
    #[test]
    fn brackets_in_literals_and_comments_do_not_count() {
        let cases = [
            r#"let s = "(((((((((((";"#,
            r#"let s = "\"((((((((((";"#,
            r##"let s = r#"(((((((((("#;"##,
            r###"let b = br##"(((((((((("##;"###,
            "// ((((((((((\nlet x = 1;",
            "/* (((((((((( */ let x = 1;",
            "/* /* (((((((((( */ */ let x = 1;",
            r#"let c = '(';"#,
            r#"let c = '\'';"#,
            // A lifetime is not a char literal: the `(` after it is real, but
            // one level is nowhere near the cap.
            r#"fn f<'a>(x: &'a str) {}"#,
        ];
        for case in cases {
            assert!(!nesting_exceeds(case, 5), "false positive on {case:?}");
        }
    }

    #[test]
    fn real_nesting_counts() {
        assert!(nesting_exceeds("let x = ((((((0));", 5));
        assert!(!nesting_exceeds("let x = ((0));", 5));
        // Mixed delimiters share one depth counter.
        assert!(nesting_exceeds("fn f() { if a { g([(1, 2)]) } }", 4));
    }

    /// Unterminated literals must not loop or panic — hostile input is not
    /// required to be valid Rust.
    #[test]
    fn unterminated_literals_terminate() {
        for case in [
            r#"let s = "unclosed ((((("#,
            r##"let s = r#"unclosed ((((("##,
            "/* unclosed ((((( ",
            "let c = '",
            "let s = \"trailing escape \\",
        ] {
            let _ = nesting_exceeds(case, 5);
        }
    }
}
