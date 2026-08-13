//! Generated documents get published — to GitHub Pages, mdBook, Docusaurus,
//! Jekyll — and everything in them that is not a label comes out of the analyzed
//! application: doc-comment prose, state names, event names, the project name.
//!
//! These tests pin the encoding rules from `docs/security.md`. They assert on
//! the *shape* of the output rather than exact strings, so they survive wording
//! changes but fail if an escape is dropped.

use crux_analyzer_i18n::Locale;
use crux_analyzer_model::{Core, Effect, Event, Machine, Project, State, StateDecl, Transition};

/// A project whose every author-controlled field is hostile.
fn hostile(doc: &str) -> Project {
    Project {
        project: "Proj".into(),
        cores: vec![Core {
            name: "App".into(),
            machines: vec![Machine {
                name: "S".into(),
                doc: Some(doc.to_string()),
                states: vec![StateDecl {
                    name: "Idle".into(),
                    doc: Some(doc.to_string()),
                    markers: vec![],
                    tags: vec![],
                    ..Default::default()
                }],
                transitions: vec![Transition {
                    from: State("Idle".into()),
                    event: Event("Go".into()),
                    to: State("Idle".into()),
                    effects: vec![Effect::bare("Render")],
                }],
                ..Default::default()
            }],
            ..Default::default()
        }],
    }
}

/// Raw HTML in a doc comment must not become a real element. GitHub sanitizes
/// rendered Markdown; most static-site pipelines do not.
#[test]
fn html_in_prose_cannot_become_an_element() {
    let doc = crux_analyzer_docgen::markdown(
        &hostile("<script>alert(1)</script> and <img src=x onerror=alert(1)>"),
        Locale::En,
    );
    assert!(!doc.contains("<script"), "{doc}");
    assert!(!doc.contains("<img"), "{doc}");
    // The text still reads, escaped.
    assert!(doc.contains("&lt;script&gt;"), "{doc}");
}

/// `Vec<String>` in prose is the case an HTML *sanitizer* would eat. Escaping
/// keeps it: `&lt;` renders back as `<`.
#[test]
fn generics_in_prose_survive() {
    let doc = crux_analyzer_docgen::markdown(&hostile("Holds a Vec<String> of ids."), Locale::En);
    assert!(doc.contains("Vec&lt;String&gt;"), "{doc}");
}

/// A fence-shaped line in prose must not close the fence around the diagram.
#[test]
fn prose_cannot_close_the_diagram_fence() {
    let doc = crux_analyzer_docgen::markdown(
        &hostile("Here is a trap:\n```\nnot really code"),
        Locale::En,
    );
    // Exactly one opening and one closing mermaid fence survive, so fences pair.
    let fences = doc
        .lines()
        .filter(|l| l.trim_start().starts_with("```"))
        .count();
    assert_eq!(fences, 2, "unbalanced fences in:\n{doc}");
    assert!(doc.contains("```mermaid"), "{doc}");
}

/// Backticks inside a Mermaid note would otherwise be able to close the fence
/// from *inside* the diagram block.
#[test]
fn backticks_in_a_note_do_not_break_out_of_the_fence() {
    let doc = crux_analyzer_docgen::markdown(&hostile("Uses ``` to fence"), Locale::En);
    let opening = doc
        .lines()
        .find(|l| l.trim_start().starts_with("`") && l.contains("mermaid"))
        .expect("a mermaid fence");
    let ticks = opening.chars().take_while(|&c| c == '`').count();
    assert!(
        ticks >= 4,
        "fence must outgrow the backticks in the body, got {ticks} in {opening:?}"
    );
}

/// A table row is one line with `|` as the column separator; the backslash has
/// to be escaped before the pipe or `\|` re-opens a column.
#[test]
fn table_cells_keep_their_column_count() {
    let doc =
        crux_analyzer_docgen::markdown(&hostile("a \\| b | c\nsecond line `unclosed"), Locale::En);
    let row = doc
        .lines()
        .find(|l| l.starts_with("| Idle |"))
        .expect("the states row");
    // Unescaped pipes only: five columns means six delimiters.
    let unescaped = row
        .char_indices()
        .filter(|(i, c)| *c == '|' && (*i == 0 || row.as_bytes()[i - 1] != b'\\'))
        .count();
    assert_eq!(unescaped, 6, "column count changed in {row:?}");
}

/// Control characters must not reach the document: a newline in a table cell
/// forges a row, and an ANSI escape survives into anything that cats the file.
#[test]
fn control_characters_are_stripped_from_cells() {
    let doc = crux_analyzer_docgen::markdown(&hostile("ansi \u{1b}[31m red \u{7}"), Locale::En);
    let row = doc
        .lines()
        .find(|l| l.starts_with("| Idle |"))
        .expect("the states row");
    assert!(!row.chars().any(char::is_control), "{row:?}");
}

/// A newline in the project name — which defaults to a *directory* name — must
/// not inject Markdown at the top of the document.
#[test]
fn the_project_name_cannot_inject_markdown() {
    let mut project = hostile("fine");
    project.project = "Proj\n\n## Injected Heading\n".into();
    let doc = crux_analyzer_docgen::markdown(&project, Locale::En);
    assert!(!doc.contains("\n## Injected Heading"), "{doc}");
    assert!(doc.starts_with("# Proj"), "{doc}");
}

// ---- Mermaid --------------------------------------------------------------

fn diagram_with_states(states: Vec<StateDecl>, transitions: Vec<Transition>) -> String {
    let project = Project {
        project: "P".into(),
        cores: vec![Core {
            name: "App".into(),
            machines: vec![Machine {
                name: "S".into(),
                states,
                transitions,
                ..Default::default()
            }],
            ..Default::default()
        }],
    };
    crux_analyzer_docgen::mermaid_diagrams(&project, Locale::En)[0]
        .mermaid
        .clone()
}

/// A variant named after a Mermaid keyword must not be emitted as a bare node
/// id — `end` alone breaks the entire diagram.
#[test]
fn keyword_state_names_get_a_generated_id() {
    let body = diagram_with_states(
        vec!["end".into(), "Idle".into()],
        vec![Transition {
            from: State("Idle".into()),
            event: Event("Stop".into()),
            to: State("end".into()),
            effects: vec![],
        }],
    );
    // The transition must not point at a bare `end`.
    let transition = body
        .lines()
        .find(|l| l.contains("-->"))
        .expect("a transition");
    assert!(
        !transition.trim_end().ends_with("--> end: Stop"),
        "bare keyword id in {transition:?}"
    );
    // The real name still shows, via a quoted label.
    assert!(body.contains("\"end\""), "{body}");
}

/// Effect names reach the diagram now that a transition label is
/// `event / action`, so they are one more identifier that must not be able to
/// inject a diagram statement or close a label.
#[test]
fn hostile_effect_names_cannot_escape_a_transition_label() {
    let body = diagram_with_states(
        vec!["Idle".into(), "Busy".into()],
        vec![Transition {
            from: State("Idle".into()),
            event: Event("Go".into()),
            to: State("Busy".into()),
            effects: vec![
                Effect::bare("Op::\"quoted\""),
                Effect::bare("Op::A\n    Idle --> Busy: Injected"),
                Effect::bare("Op::%% commented"),
            ],
        }],
    );

    // `[*]` arrows state the derived roles — they carry no name from the
    // analyzed application, so only the labelled lines are under test here.
    let transitions: Vec<&str> = body
        .lines()
        .filter(|l| l.contains("-->") && !l.contains("[*]"))
        .collect();
    assert_eq!(transitions.len(), 1, "a statement was injected:\n{body}");
    let label = transitions[0];
    assert!(!label.contains('"'), "unescaped quote in {label:?}");
    assert!(!label.contains("%%"), "live comment in {label:?}");
    // The names still read, through Mermaid's entity codes.
    assert!(label.contains("#quot;quoted#quot;"), "{label}");
}

/// A composite leaf flattens to `Parent_Child`; a sibling variant literally
/// named `Parent_Child` must not silently merge into the same node.
#[test]
fn flattened_composites_do_not_collide_with_real_names() {
    let body = diagram_with_states(
        vec!["Active/Loading".into(), "Active_Loading".into()],
        vec![],
    );
    let ids: Vec<&str> = body
        .lines()
        .filter_map(|l| l.trim().strip_prefix("state \""))
        .filter_map(|l| l.split(" as ").nth(1))
        .collect();
    let mut unique = ids.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(ids.len(), unique.len(), "ids collided in:\n{body}");
}

/// A raw identifier is a legal variant name and `r#type` is not a legal Mermaid
/// id.
#[test]
fn raw_identifier_state_names_are_usable() {
    let body = diagram_with_states(vec!["r#type".into()], vec![]);
    assert!(!body.contains("r#type\n"), "{body}");
    assert!(body.contains("\"r#type\""), "label missing in {body}");
}

/// Every Mermaid statement is line-terminated and `%%` starts a comment, so
/// neither may survive into a note.
#[test]
fn notes_cannot_inject_diagram_lines() {
    let body = diagram_with_states(
        vec![StateDecl {
            name: "Idle".into(),
            doc: Some("first\nstate Injected\n%% commented".into()),
            markers: vec![],
            tags: vec![],
            ..Default::default()
        }],
        vec![],
    );
    let note = body
        .lines()
        .find(|l| l.contains("note right of"))
        .expect("a note");
    assert!(!note.contains("%%"), "{note:?}");
    assert!(
        !body.lines().any(|l| l.trim() == "state Injected"),
        "prose became a diagram statement:\n{body}"
    );
}

/// A quote in prose would close a quoted label.
#[test]
fn quotes_in_prose_cannot_close_a_label() {
    let body = diagram_with_states(
        vec![StateDecl {
            name: "Idle".into(),
            doc: Some("says \"hello\" loudly".into()),
            markers: vec![],
            tags: vec![],
            ..Default::default()
        }],
        vec![],
    );
    let note = body
        .lines()
        .find(|l| l.contains("note right of"))
        .expect("a note");
    assert!(!note.contains('"'), "raw quote survived in {note:?}");
    assert!(note.contains("#quot;"), "{note:?}");
}

/// The Markdown parts of the document: everything outside a fenced block.
///
/// The diagram is excluded deliberately. Mermaid note text is *text* — Mermaid
/// does not turn it into a link — so a scheme surviving there is not a clickable
/// payload, and neutralizing it would be a transformation with no threat behind
/// it. What must be defused is a real Markdown link target.
fn outside_fences(doc: &str) -> String {
    let mut out = String::new();
    let mut inside = false;
    for line in doc.lines() {
        if line.trim_start().starts_with("```") {
            inside = !inside;
            continue;
        }
        if !inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// A Markdown link is preserved on purpose, so its *target* is what needs
/// checking: `javascript:` in a published document is a clickable payload in any
/// renderer that does not sanitize.
#[test]
fn markdown_link_targets_drop_unsafe_schemes() {
    let doc = crux_analyzer_docgen::markdown(
        &hostile(
            "[a](javascript:alert(1)) [b](JavaScript:alert(1)) [c](data:text/html,x) \
             [d](vbscript:x) [e](https://ok.example) [f](/relative) [g](#anchor) \
             ![img](javascript:alert(1))",
        ),
        Locale::En,
    );
    let markdown = outside_fences(&doc);
    for scheme in ["javascript:", "JavaScript:", "data:", "vbscript:"] {
        assert!(
            !markdown.contains(scheme),
            "{scheme} survived in:\n{markdown}"
        );
    }
    // The colon is escaped, not deleted: the reader still sees the text.
    assert!(markdown.contains("javascript&#58;"), "{markdown}");
    // Safe and relative targets are untouched.
    assert!(markdown.contains("[e](https://ok.example)"), "{markdown}");
    assert!(markdown.contains("[f](/relative)"), "{markdown}");
    assert!(markdown.contains("[g](#anchor)"), "{markdown}");
}

/// Non-ASCII prose must survive the URL scan byte-for-byte — the scan slices on
/// ASCII markers, and a byte-wise copy would mangle UTF-8.
#[test]
fn non_ascii_prose_survives_url_neutralization() {
    let doc = crux_analyzer_docgen::markdown(
        &hostile("Gravação começou — ação não permitida. [x](javascript:1) 日本語"),
        Locale::En,
    );
    assert!(
        doc.contains("Gravação começou — ação não permitida."),
        "{doc}"
    );
    assert!(doc.contains("日本語"), "{doc}");
    assert!(!outside_fences(&doc).contains("javascript:"), "{doc}");
}
