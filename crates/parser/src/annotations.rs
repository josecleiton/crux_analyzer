//! Reads `///` documentation off items and splits it into prose and declared
//! annotations.
//!
//! An annotation is an `@` line written *inside* a doc comment. That is the
//! only mechanism that needs zero dependencies in the analyzed crate:
//! crux_analyzer must never be a dependency of the apps it reads, so a real
//! proc-macro attribute is out and a bare unknown attribute would not compile.
//! Doc comments always compile, and they still render in `rustdoc`.
//!
//! # What counts as an annotation
//!
//! A line whose trimmed text starts with `@` is annotation *syntax*. When it
//! is one of the recognized forms it is consumed and removed from the prose;
//! when it is not, it is removed **and reported**, because a silently inert
//! `@failur` is exactly the kind of quiet wrong answer the honesty rule exists
//! to prevent. A line where `@` is not the first character is ordinary prose
//! and is never touched — that is what keeps `` `@Generable` `` in the middle
//! of a sentence, or an email address, safe. Fenced code blocks are skipped
//! wholesale, and `\@` at the start of a line is the escape hatch.

use crux_analyzer_model::Marker;
use syn::spanned::Spanned;

/// One line of documentation, with the source line it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DocLine {
    pub line: usize,
    pub text: String,
}

/// An annotation-shaped line that is not a recognized annotation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AnnotationProblem {
    pub line: usize,
    /// The offending text, trimmed — interpolated into the warning verbatim.
    pub text: String,
}

/// The documentation authored on one item.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DocBlock {
    /// The prose, annotation lines removed. `None` when nothing remains.
    pub doc: Option<String>,
    /// Declared markers, in first-seen order, deduplicated.
    pub markers: Vec<Marker>,
    /// Declared `@tag` names, in first-seen order, deduplicated.
    pub tags: Vec<String>,
    /// Annotation-shaped lines we could not recognize.
    pub problems: Vec<AnnotationProblem>,
}

impl DocBlock {
    /// Merges a composite parent's documentation into a child leaf's.
    ///
    /// Markers and tags **union**, parent first: a marker on a superstate is a
    /// statement about the whole region, so the child inherits it. Prose
    /// **concatenates** rather than the child simply winning — a composite
    /// parent has no node of its own in `states[]`, so dropping its prose
    /// whenever the child has some would silently lose what the author wrote.
    pub fn inherit(&self, parent: &DocBlock) -> DocBlock {
        let doc = match (&parent.doc, &self.doc) {
            (Some(parent_doc), Some(child_doc)) => Some(format!("{parent_doc}\n\n{child_doc}")),
            (Some(parent_doc), None) => Some(parent_doc.clone()),
            (None, child) => child.clone(),
        };
        let mut markers = parent.markers.clone();
        for marker in &self.markers {
            push_unique(&mut markers, *marker);
        }
        let mut tags = parent.tags.clone();
        for tag in &self.tags {
            push_unique(&mut tags, tag.clone());
        }
        // Problems stay with the item that wrote them, so a parent's typo is
        // reported once rather than once per child.
        DocBlock {
            doc,
            markers,
            tags,
            problems: self.problems.clone(),
        }
    }
}

/// The documentation authored on an item, from its attributes.
pub(crate) fn doc_block(attrs: &[syn::Attribute]) -> DocBlock {
    split_annotations(&dedent(doc_lines(attrs)))
}

/// The `#[doc]` attributes of an item as individual lines.
///
/// `///` desugars to one attribute per line, while `/** … */` is a single
/// attribute whose value contains newlines; splitting every value on `\n`
/// makes both go through the same path, and `#[doc = "…"]` written by hand
/// needs no special case at all.
fn doc_lines(attrs: &[syn::Attribute]) -> Vec<DocLine> {
    let mut lines = Vec::new();
    for attr in attrs {
        if !attr.path().is_ident("doc") {
            // Also skips `#[doc(hidden)]`, which is a `Meta::List`.
            continue;
        }
        let syn::Meta::NameValue(name_value) = &attr.meta else {
            continue;
        };
        let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(text),
            ..
        }) = &name_value.value
        else {
            continue;
        };
        let start = attr.span().start().line;
        for (offset, line) in text.value().split('\n').enumerate() {
            lines.push(DocLine {
                line: start + offset,
                text: line.to_string(),
            });
        }
    }
    lines
}

/// Removes the indentation common to every non-blank line — rustdoc's rule,
/// which for `///` means dropping the conventional single space.
///
/// `*` prefixes are deliberately left alone: rustdoc does not strip them
/// either, and doing so would mangle a line that legitimately starts with
/// `* emphasis`.
fn dedent(lines: Vec<DocLine>) -> Vec<DocLine> {
    let indent = lines
        .iter()
        .filter(|line| !line.text.trim().is_empty())
        .map(|line| line.text.len() - line.text.trim_start().len())
        .min()
        .unwrap_or(0);
    if indent == 0 {
        return lines;
    }
    lines
        .into_iter()
        .map(|line| DocLine {
            line: line.line,
            text: line.text.chars().skip(indent).collect(),
        })
        .collect()
}

/// Splits dedented documentation into prose and annotations.
///
/// Pure: the grammar lives here so it can be tested with plain strings.
fn split_annotations(lines: &[DocLine]) -> DocBlock {
    let mut block = DocBlock::default();
    let mut prose: Vec<String> = Vec::new();
    let mut in_fence = false;

    for line in lines {
        let trimmed = line.text.trim();

        if is_fence(trimmed) {
            in_fence = !in_fence;
            prose.push(line.text.trim_end().to_string());
            continue;
        }

        // Inside a code fence everything is a sample, including a line that
        // happens to look like an annotation.
        if in_fence {
            prose.push(line.text.trim_end().to_string());
            continue;
        }

        // The escape hatch for prose that must start with a literal `@`.
        if let Some(escaped) = trimmed.strip_prefix("\\@") {
            prose.push(format!("@{escaped}"));
            continue;
        }

        if !trimmed.starts_with('@') {
            prose.push(line.text.trim_end().to_string());
            continue;
        }

        match parse_annotation(trimmed) {
            Some(Annotation::Marker(marker)) => push_unique(&mut block.markers, marker),
            Some(Annotation::Tags(tags)) => {
                for tag in tags {
                    push_unique(&mut block.tags, tag);
                }
            }
            None => block.problems.push(AnnotationProblem {
                line: line.line,
                text: trimmed.to_string(),
            }),
        }
    }

    block.doc = join_prose(prose);
    block
}

enum Annotation {
    Marker(Marker),
    Tags(Vec<String>),
}

/// Recognizes one annotation line, which is already known to start with `@`.
///
/// Returns `None` for anything not recognized — the caller turns that into a
/// warning rather than guessing what was meant.
fn parse_annotation(trimmed: &str) -> Option<Annotation> {
    let body = trimmed.strip_prefix('@')?;
    let keyword_len = body
        .char_indices()
        .take_while(|(index, c)| {
            if *index == 0 {
                c.is_ascii_alphabetic()
            } else {
                c.is_ascii_alphanumeric() || *c == '_' || *c == '-'
            }
        })
        .count();
    if keyword_len == 0 {
        return None;
    }
    let (keyword, rest) = body.split_at(keyword_len);
    let rest = rest.trim();

    // Keywords match case-insensitively, so a capitalization slip just works
    // instead of becoming a warning.
    match keyword.to_ascii_lowercase().as_str() {
        "failure" if rest.is_empty() => Some(Annotation::Marker(Marker::Failure)),
        "deprecated" if rest.is_empty() => Some(Annotation::Marker(Marker::Deprecated)),
        "tag" => parse_tags(rest).map(Annotation::Tags),
        _ => None,
    }
}

/// `@tag a, b` → `["a", "b"]`. `None` when a name is missing or malformed, so
/// the whole line is reported instead of a half-understood tag being kept.
fn parse_tags(rest: &str) -> Option<Vec<String>> {
    let names: Vec<&str> = rest
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|name| !name.is_empty())
        .collect();
    if names.is_empty() {
        return None;
    }
    if !names.iter().all(|name| {
        name.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    }) {
        return None;
    }
    Some(names.into_iter().map(str::to_string).collect())
}

fn is_fence(trimmed: &str) -> bool {
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

/// Trims the edges and collapses runs of blank lines.
///
/// The collapse is what repairs the hole an annotation leaves behind: a tag
/// written between two paragraphs yields exactly the same prose as one written
/// at the end. Lines are never reflowed — the author's wrapping is theirs.
fn join_prose(lines: Vec<String>) -> Option<String> {
    let mut out: Vec<String> = Vec::new();
    for line in lines {
        let blank = line.trim().is_empty();
        if blank && out.last().is_none_or(|last| last.trim().is_empty()) {
            continue;
        }
        out.push(line);
    }
    while out.last().is_some_and(|last| last.trim().is_empty()) {
        out.pop();
    }
    if out.is_empty() {
        return None;
    }
    Some(out.join("\n"))
}

fn push_unique<T: PartialEq>(values: &mut Vec<T>, value: T) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds doc lines the way `///` would: one per line, each with the
    /// conventional leading space, numbered from 1.
    fn doc(text: &str) -> DocBlock {
        let lines = text
            .lines()
            .enumerate()
            .map(|(index, line)| DocLine {
                line: index + 1,
                text: format!(" {line}"),
            })
            .collect::<Vec<_>>();
        split_annotations(&dedent(lines))
    }

    #[test]
    fn keeps_prose_and_drops_nothing_when_there_are_no_annotations() {
        let block = doc("Nothing is being recorded yet.");
        assert_eq!(block.doc.as_deref(), Some("Nothing is being recorded yet."));
        assert!(block.markers.is_empty());
        assert!(block.tags.is_empty());
        assert!(block.problems.is_empty());
    }

    #[test]
    fn recognizes_the_three_annotations() {
        let block = doc("It broke.\n\n@failure\n@deprecated\n@tag retryable");
        assert_eq!(block.doc.as_deref(), Some("It broke."));
        assert_eq!(block.markers, [Marker::Failure, Marker::Deprecated]);
        assert_eq!(block.tags, ["retryable"]);
        assert!(block.problems.is_empty());
    }

    /// The equivalence that the blank-line collapse exists for.
    #[test]
    fn an_annotation_in_the_middle_reads_like_one_at_the_end() {
        let middle = doc("First paragraph.\n\n@tag retryable\n\nSecond paragraph.");
        let trailing = doc("First paragraph.\n\nSecond paragraph.\n\n@tag retryable");
        assert_eq!(middle.doc.as_deref(), Some("First paragraph.\n\nSecond paragraph."));
        assert_eq!(middle.doc, trailing.doc);
        assert_eq!(middle.tags, trailing.tags);
    }

    #[test]
    fn keeps_paragraph_breaks_and_the_authors_wrapping() {
        let block = doc("One line\nwrapped by hand.\n\nA second paragraph.");
        assert_eq!(
            block.doc.as_deref(),
            Some("One line\nwrapped by hand.\n\nA second paragraph.")
        );
    }

    #[test]
    fn an_at_sign_mid_prose_is_not_an_annotation() {
        // Verbatim from the private corpus (`insight.rs`).
        let line = "Apple constrains the shape structurally — `@Generable` leaves the model no";
        let block = doc(line);
        assert_eq!(block.doc.as_deref(), Some(line));
        assert!(block.problems.is_empty());

        let email = doc("Ask support@example.com for help.");
        assert_eq!(email.doc.as_deref(), Some("Ask support@example.com for help."));
        assert!(email.problems.is_empty());
    }

    #[test]
    fn keywords_are_case_insensitive() {
        let block = doc("@FAILURE\n@Deprecated\n@Tag Retryable");
        assert_eq!(block.markers, [Marker::Failure, Marker::Deprecated]);
        assert_eq!(block.tags, ["Retryable"]);
        assert!(block.problems.is_empty());
    }

    #[test]
    fn tags_split_on_whitespace_and_commas() {
        let block = doc("@tag retryable, offline\n@tag manual-resolution");
        assert_eq!(block.tags, ["retryable", "offline", "manual-resolution"]);
    }

    #[test]
    fn markers_and_tags_are_deduplicated_in_first_seen_order() {
        let block = doc("@tag b\n@failure\n@tag a\n@failure\n@tag b");
        assert_eq!(block.markers, [Marker::Failure]);
        assert_eq!(block.tags, ["b", "a"]);
    }

    #[test]
    fn an_unrecognized_annotation_is_reported_and_stripped() {
        let block = doc("It broke.\n@failur");
        assert_eq!(block.doc.as_deref(), Some("It broke."));
        assert!(block.markers.is_empty());
        assert_eq!(
            block.problems,
            [AnnotationProblem {
                line: 2,
                text: "@failur".into()
            }]
        );
    }

    #[test]
    fn a_marker_that_takes_no_argument_is_reported_when_given_one() {
        let block = doc("@failure because the disk was full");
        assert!(block.markers.is_empty());
        assert_eq!(block.problems.len(), 1);
        assert_eq!(block.problems[0].text, "@failure because the disk was full");
    }

    #[test]
    fn a_tag_without_a_usable_name_is_reported() {
        for line in ["@tag", "@tag bad name!", "@tag ()"] {
            let block = doc(line);
            assert!(block.tags.is_empty(), "{line}");
            assert_eq!(block.problems.len(), 1, "{line}");
        }
    }

    #[test]
    fn annotations_inside_a_code_fence_are_not_annotations() {
        let block = doc("How to mark one:\n\n```rust\n/// @failure\nFailed,\n```");
        assert!(block.markers.is_empty());
        assert!(block.problems.is_empty());
        assert_eq!(
            block.doc.as_deref(),
            Some("How to mark one:\n\n```rust\n/// @failure\nFailed,\n```")
        );
    }

    #[test]
    fn a_backslash_escapes_a_leading_at_sign() {
        let block = doc("\\@failure is how you mark a failure.");
        assert!(block.markers.is_empty());
        assert!(block.problems.is_empty());
        assert_eq!(
            block.doc.as_deref(),
            Some("@failure is how you mark a failure.")
        );
    }

    #[test]
    fn an_annotation_only_doc_comment_leaves_no_prose() {
        let block = doc("@failure");
        assert!(block.doc.is_none());
        assert_eq!(block.markers, [Marker::Failure]);
    }

    #[test]
    fn empty_documentation_produces_an_empty_block() {
        assert_eq!(doc(""), DocBlock::default());
        assert_eq!(doc("\n   \n"), DocBlock::default());
    }

    #[test]
    fn a_bare_at_sign_is_reported_rather_than_guessed() {
        let block = doc("@");
        assert_eq!(block.problems.len(), 1);
        assert!(block.doc.is_none());
    }

    #[test]
    fn inherit_unions_markers_and_concatenates_prose_parent_first() {
        let parent = doc("A session is live.\n@deprecated\n@tag region");
        let child = doc("Fetching the manifest.\n@failure\n@tag region\n@tag leaf");
        let merged = child.inherit(&parent);

        assert_eq!(
            merged.doc.as_deref(),
            Some("A session is live.\n\nFetching the manifest.")
        );
        assert_eq!(merged.markers, [Marker::Deprecated, Marker::Failure]);
        assert_eq!(merged.tags, ["region", "leaf"]);
    }

    #[test]
    fn inherit_falls_back_to_either_side_when_only_one_documents() {
        let documented = doc("Only the parent says something.");
        let empty = DocBlock::default();
        assert_eq!(empty.inherit(&documented).doc, documented.doc);
        assert_eq!(documented.inherit(&empty).doc, documented.doc);
        assert!(empty.inherit(&empty).doc.is_none());
    }

    #[test]
    fn reads_an_explicit_doc_attribute() {
        let item: syn::ItemEnum = syn::parse_str(
            r#"enum State {
                #[doc = "Nothing yet."]
                #[doc = "@failure"]
                Idle,
            }"#,
        )
        .unwrap();
        let block = doc_block(&item.variants[0].attrs);
        assert_eq!(block.doc.as_deref(), Some("Nothing yet."));
        assert_eq!(block.markers, [Marker::Failure]);
    }

    #[test]
    fn reads_and_dedents_a_block_doc_comment() {
        let item: syn::ItemEnum = syn::parse_str(
            "enum State {
                /** Nothing yet.

                    @failure
                */
                Idle,
            }",
        )
        .unwrap();
        let block = doc_block(&item.variants[0].attrs);
        assert_eq!(block.markers, [Marker::Failure]);
        assert_eq!(block.doc.as_deref(), Some("Nothing yet."));
    }

    #[test]
    fn ignores_doc_hidden_and_other_attributes() {
        let item: syn::ItemEnum = syn::parse_str(
            r#"enum State {
                /// Real prose.
                #[doc(hidden)]
                #[default]
                Idle,
            }"#,
        )
        .unwrap();
        let block = doc_block(&item.variants[0].attrs);
        assert_eq!(block.doc.as_deref(), Some("Real prose."));
        assert!(block.problems.is_empty());
    }

    #[test]
    fn reports_the_source_line_of_a_problem() {
        let item: syn::ItemEnum = syn::parse_str(
            "enum State {
                /// Prose.
                ///
                /// @nonsense
                Idle,
            }",
        )
        .unwrap();
        let block = doc_block(&item.variants[0].attrs);
        assert_eq!(block.problems.len(), 1);
        assert_eq!(block.problems[0].line, 4);
    }
}
