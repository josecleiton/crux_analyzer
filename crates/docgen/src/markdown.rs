//! Markdown generator: one document with a Mermaid diagram, a states table
//! and a transition table per machine.

use crux_analyzer_i18n::Locale;
use crux_analyzer_model::{Machine, Project, State, StateDecl};

use crate::{effects_cell, has_documented_states, machine_diagram, marker_label, Labels};

pub fn markdown(project: &Project, locale: Locale) -> String {
    let labels = Labels::for_locale(locale);
    let mut out = String::new();
    push_line(&mut out, &format!("# {}", project.project));

    for core in &project.cores {
        push_line(&mut out, "");
        push_line(&mut out, &format!("## {}: {}", labels.core, core.name));

        for machine in &core.machines {
            push_line(&mut out, "");
            push_line(
                &mut out,
                &format!("### {}: {}", labels.machine, machine.name),
            );

            // The machine's own description, verbatim: Markdown handles
            // multi-paragraph prose natively, so only table cells need
            // flattening.
            if let Some(doc) = &machine.doc {
                push_line(&mut out, "");
                push_line(&mut out, doc.trim());
            }

            push_line(&mut out, "");
            push_line(&mut out, "```mermaid");
            push_line(&mut out, &machine_diagram(machine, &labels));
            push_line(&mut out, "```");

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
    }

    out
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

    push_line(out, "");
    push_line(out, &format!("#### {}", labels.states));
    push_line(out, "");
    push_line(
        out,
        &format!(
            "| {} | {} | {} | {} |",
            labels.state, labels.description, labels.markers, labels.tags
        ),
    );
    push_line(out, "| --- | --- | --- | --- |");
    for state in &machine.states {
        push_line(
            out,
            &format!(
                "| {} | {} | {} | {} |",
                state.name,
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
        push_line(out, &format!("##### {}", state.name));
        push_line(out, "");
        push_line(out, doc.trim());
    }
}

fn description_cell(state: &StateDecl, labels: &Labels) -> String {
    match &state.doc {
        Some(doc) => table_cell(paragraphs(doc).first().copied().unwrap_or_default()),
        None => labels.no_value.to_string(),
    }
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
fn table_cell(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").replace('|', "\\|")
}

fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}
