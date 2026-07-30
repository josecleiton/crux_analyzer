//! Markdown generator: one document with a Mermaid diagram, a states table
//! and a transition table per machine.

use std::collections::{BTreeMap, BTreeSet};

use crux_analyzer_i18n::Locale;
use crux_analyzer_model::{DocumentedName, Machine, Project, State, StateDecl};

use crate::{
    effects_cell, has_documented_states, machine_diagram, marker_label, one_line, Labels,
    MachineRoles,
};

pub fn markdown(project: &Project, locale: Locale) -> String {
    let labels = Labels::for_locale(locale);
    let mut out = String::new();
    // The project name defaults to a *directory* name, which can hold anything
    // a filesystem allows — newlines included.
    push_line(&mut out, &format!("# {}", one_line(&project.project)));

    for core in &project.cores {
        push_line(&mut out, "");
        push_line(&mut out, &format!("## {}: {}", labels.core, core.name));

        for machine in &core.machines {
            push_line(&mut out, "");
            push_line(
                &mut out,
                &format!("### {}: {}", labels.machine, machine.name),
            );

            // The machine's own description as a block: Markdown handles
            // multi-paragraph prose natively, so only table cells need
            // flattening — but prose is untrusted text either way.
            if let Some(doc) = &machine.doc {
                push_line(&mut out, "");
                push_line(&mut out, &prose_block(doc.trim()));
            }
            push_machine_annotations(&mut out, machine, &labels);

            let diagram = machine_diagram(machine, &labels);
            let fence = fence_for(&diagram);
            push_line(&mut out, "");
            push_line(&mut out, &format!("{fence}mermaid"));
            push_line(&mut out, &diagram);
            push_line(&mut out, &fence);

            push_states(&mut out, machine, &labels);

            push_line(&mut out, "");
            push_line(
                &mut out,
                &format!(
                    "| {} | {} | {} | {} |",
                    labels.from, labels.event, labels.to, labels.effects
                ),
            );
            push_line(&mut out, "| --- | --- | --- | --- |");
            for transition in &machine.transitions {
                let from = if transition.from.0 == State::ANY {
                    labels.any_source.to_string()
                } else {
                    transition.from.0.clone()
                };
                push_line(
                    &mut out,
                    &format!(
                        "| {from} | `{}` | {} | {} |",
                        transition.event.0,
                        transition.to.0,
                        effects_cell(transition, &labels),
                    ),
                );
            }
        }

        // What this core asks of the shell, gathered from its requests.
        push_capabilities(&mut out, core, &labels);

        // The core's documented events and effects close its section: the
        // vocabulary already appears in the transition tables above, so these
        // catalogs only exist where an author explained something.
        push_catalog(&mut out, labels.events, labels.event, &core.events, &labels);
        push_catalog(&mut out, labels.effects, labels.effect, &core.effects, &labels);
    }

    out
}

/// What the core needs from the shell: one row per capability, the operations
/// requested through it, and the events those requests come back as.
///
/// Read off the requests themselves, so it says nothing the transition tables
/// do not already contain — it just answers a question they answer badly
/// ("what does this core talk to?"). Omitted entirely when no request resolved
/// to a capability, which is also the pre-capability output.
fn push_capabilities(out: &mut String, core: &crux_analyzer_model::Core, labels: &Labels) {
    // BTreeMap/BTreeSet: deterministic rows and columns, and one entry per name
    // however many transitions requested it.
    let mut by_capability: BTreeMap<&str, (BTreeSet<&str>, BTreeSet<&str>)> = BTreeMap::new();
    for effect in core
        .machines
        .iter()
        .flat_map(|machine| &machine.transitions)
        .flat_map(|transition| &transition.effects)
    {
        let Some(capability) = &effect.capability else {
            continue;
        };
        let entry = by_capability.entry(capability.as_str()).or_default();
        entry.0.insert(effect.name.as_str());
        for event in &effect.resolves_with {
            entry.1.insert(event.0.as_str());
        }
    }
    if by_capability.is_empty() {
        return;
    }

    push_line(out, "");
    push_line(out, &format!("### {}", labels.capabilities));
    push_line(out, "");
    push_line(
        out,
        &format!(
            "| {} | {} | {} |",
            labels.capability, labels.operations, labels.answers
        ),
    );
    push_line(out, "| --- | --- | --- |");
    for (capability, (operations, answers)) in by_capability {
        let answers = if answers.is_empty() {
            labels.no_value.to_string()
        } else {
            monospace_list(answers.into_iter())
        };
        push_line(
            out,
            &format!(
                "| `{}` | {} | {} |",
                capability,
                monospace_list(operations.into_iter()),
                answers
            ),
        );
    }
}

/// Identifiers as a monospace, comma-separated cell.
fn monospace_list<'a>(names: impl Iterator<Item = &'a str>) -> String {
    names
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A name/description table for documented events or effects. Names are
/// monospace — identifiers from the analyzed app — and the whole description
/// is flattened into the cell (nothing dropped, just unwrapped).
fn push_catalog(
    out: &mut String,
    heading: &str,
    name_column: &str,
    entries: &[DocumentedName],
    labels: &Labels,
) {
    if entries.is_empty() {
        return;
    }
    push_line(out, "");
    push_line(out, &format!("### {heading}"));
    push_line(out, "");
    push_line(out, &format!("| {} | {} |", name_column, labels.description));
    push_line(out, "| --- | --- |");
    for entry in entries {
        push_line(
            out,
            &format!("| `{}` | {} |", entry.name, table_cell(&entry.doc)),
        );
    }
}

/// Markers and tags declared on the state enum itself. They describe the whole
/// region, so they belong beside its description rather than in the per-state
/// table.
fn push_machine_annotations(out: &mut String, machine: &Machine, labels: &Labels) {
    if !machine.markers.is_empty() {
        push_line(out, "");
        push_line(
            out,
            &format!(
                "**{}:** {}",
                labels.markers,
                machine
                    .markers
                    .iter()
                    .map(|marker| marker_label(*marker, labels))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
    if !machine.tags.is_empty() {
        push_line(out, "");
        push_line(
            out,
            &format!(
                "**{}:** {}",
                labels.tags,
                machine
                    .tags
                    .iter()
                    .map(|tag| format!("`{tag}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        );
    }
}

/// The states table, plus a section per state whose description does not fit
/// one paragraph.
///
/// Emitted only when the source documented something: otherwise every existing
/// document would grow a column of em dashes for no information.
fn push_states(out: &mut String, machine: &Machine, labels: &Labels) {
    if !has_documented_states(machine) {
        return;
    }

    let roles = MachineRoles::of(machine);
    push_line(out, "");
    push_line(out, &format!("#### {}", labels.states));
    push_line(out, "");
    push_line(
        out,
        &format!(
            "| {} | {} | {} | {} | {} |",
            labels.state, labels.role, labels.description, labels.markers, labels.tags
        ),
    );
    push_line(out, "| --- | --- | --- | --- | --- |");
    for state in &machine.states {
        push_line(
            out,
            &format!(
                "| {} | {} | {} | {} | {} |",
                state.name,
                role_cell(&roles, &state.name, labels),
                description_cell(state, labels),
                markers_cell(state, labels),
                tags_cell(state, labels),
            ),
        );
    }

    // A table cell is one line, so anything past the first paragraph would be
    // lost. Those states get their prose back verbatim, below the table.
    for state in &machine.states {
        let Some(doc) = &state.doc else { continue };
        if paragraphs(doc).len() < 2 {
            continue;
        }
        push_line(out, "");
        push_line(out, &format!("##### {}", one_line(&state.name)));
        push_line(out, "");
        push_line(out, &prose_block(doc.trim()));
    }
}

fn description_cell(state: &StateDecl, labels: &Labels) -> String {
    match &state.doc {
        Some(doc) => table_cell(paragraphs(doc).first().copied().unwrap_or_default()),
        None => labels.no_value.to_string(),
    }
}

/// The derived roles as localized words, in the order a reader walks a machine:
/// where it starts, where it ends. Kept out of the markers cell on purpose —
/// that column is what the *author* declared, this one is what the graph says.
fn role_cell(roles: &MachineRoles, state: &str, labels: &Labels) -> String {
    let mut words = Vec::new();
    if roles.is_initial(state) {
        words.push(labels.role_initial);
    }
    if roles.is_final(state) {
        words.push(labels.role_final);
    }
    if words.is_empty() {
        return labels.no_value.to_string();
    }
    words.join(", ")
}

/// Markers as localized words — this is crux_analyzer's own vocabulary.
fn markers_cell(state: &StateDecl, labels: &Labels) -> String {
    if state.markers.is_empty() {
        return labels.no_value.to_string();
    }
    state
        .markers
        .iter()
        .map(|marker| marker_label(*marker, labels))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Tags in monospace and untranslated — they are data from the analyzed app.
fn tags_cell(state: &StateDecl, labels: &Labels) -> String {
    if state.tags.is_empty() {
        return labels.no_value.to_string();
    }
    state
        .tags
        .iter()
        .map(|tag| format!("`{tag}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn paragraphs(doc: &str) -> Vec<&str> {
    doc.split("\n\n").map(str::trim).filter(|p| !p.is_empty()).collect()
}

/// Escapes author prose for a Markdown table cell: a row is one line, and a
/// bare `|` would open a new column.
///
/// The backslash goes first, or prose containing a literal `\|` would become
/// `\\|` — a rendered backslash followed by an *unescaped* pipe, which opens a
/// column anyway.
///
/// Backticks are deliberately *not* escaped. A cell holds the same author
/// Markdown a `prose_block` does, where backticks are a feature, and a row
/// cannot be spilled by one: a table row is split on its unescaped pipes
/// before its cells are parsed as inline content, so a backtick never crosses
/// into the next column, and an unpaired one is already literal where it
/// stands. Escaping them turned every documented `field` into a visible
/// `` \`field\` ``. See `docs/security.md`.
fn table_cell(text: &str) -> String {
    let flat: String = text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_control())
        .collect();
    let escaped = flat
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\\', "\\\\")
        .replace('|', "\\|");
    // A cell holds the same author Markdown a prose block does, links included.
    // Last, so the `&#58;` it may emit is not re-escaped by the `&` pass above.
    neutralize_unsafe_urls(&escaped)
}

/// Author prose as a Markdown block, with the two break-outs neutralized.
///
/// Author Markdown is a *feature* — `**bold**`, lists and backticks are meant to
/// render, here and in the web UI — so this is deliberately not an escape of
/// Markdown syntax. What it removes is the ability to leave Markdown:
///
/// - `<` becomes `&lt;`, so raw HTML in a doc comment cannot become a real
///   element in a published document. GitHub sanitizes rendered Markdown, but
///   mdBook, Docusaurus, Jekyll and VitePress generally do not. `Vec<String>`
///   in prose still renders as `Vec<String>`.
/// - a fence-shaped line (```` ``` ````, `~~~`) is indented so it cannot close
///   the fence this document opened around a diagram.
///
/// See `docs/security.md`.
fn prose_block(text: &str) -> String {
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let fenced = escaped
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                format!("&#96;{}", &trimmed[1..])
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    neutralize_unsafe_urls(&fenced)
}

/// URL schemes a Markdown link or image in author prose may use.
const SAFE_SCHEMES: &[&str] = &["http", "https", "mailto"];

/// Defuses `[text](javascript:…)` and friends.
///
/// Escaping `<` kills raw HTML and autolinks, but a *Markdown* link is preserved
/// on purpose — author Markdown is a feature — and `[click](javascript:alert(1))`
/// renders as a working `<a href="javascript:…">` in any renderer that does not
/// sanitize. The web UI refuses those schemes at render time
/// (`StateDoc.tsx`); a published document has no such layer, so the generator
/// has to.
///
/// The scheme's colon becomes `&#58;`, which leaves every visible character in
/// place while turning the target into an inert relative path. Deliberately not
/// a Markdown parser and deliberately no `regex` dependency: it only has to
/// recognize a scheme, and over-escaping a colon in a link target is harmless.
fn neutralize_unsafe_urls(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    // Everything before this index has been copied. Prose is UTF-8 and the
    // markers are ASCII, so slicing at a marker is always on a char boundary.
    let mut copied = 0;
    let mut i = 0;

    while i + 1 < bytes.len() {
        // A link target opens with `](` (inline) or `]:` (reference definition).
        if bytes[i] != b']' || !matches!(bytes[i + 1], b'(' | b':') {
            i += 1;
            continue;
        }

        let inline = bytes[i + 1] == b'(';
        let target_start = i + 2;
        // The target runs to the closing paren, or to end of line for a
        // reference definition.
        let end = bytes[target_start..]
            .iter()
            .position(|&b| b == b'\n' || (inline && b == b')'))
            .map_or(bytes.len(), |offset| target_start + offset);

        out.push_str(&text[copied..target_start]);
        out.push_str(&defuse_scheme(&text[target_start..end]));
        copied = end;
        i = end;
    }

    out.push_str(&text[copied..]);
    out
}

/// Escapes the scheme colon of `target` unless the scheme is allowed. Relative
/// targets, anchors and scheme-less paths pass through untouched.
fn defuse_scheme(target: &str) -> String {
    let trimmed = target.trim_start();
    let Some(colon) = trimmed.find(':') else {
        return target.to_string();
    };
    // A colon that comes after a path separator is not a scheme.
    let scheme = &trimmed[..colon];
    if scheme.contains(['/', '?', '#', ' ']) {
        return target.to_string();
    }
    if SAFE_SCHEMES.contains(&scheme.to_ascii_lowercase().as_str()) {
        return target.to_string();
    }
    target.replacen(':', "&#58;", 1)
}

/// A fence long enough to contain `body`.
///
/// A state or event name cannot contain a backtick, but a doc comment reaching a
/// Mermaid note can — and three of them would close the fence and drop the rest
/// of the diagram into the document as prose.
fn fence_for(body: &str) -> String {
    let longest_run = body
        .split(|c| c != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    "`".repeat(longest_run.max(2) + 1)
}

fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}
