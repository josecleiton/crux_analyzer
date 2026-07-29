//! Documentation generators for the crux_analyzer semantic model.
//!
//! Every generator consumes only [`crux_analyzer_model`] types — never the
//! parser or its AST — so they work for any client that has a model JSON.
//!
//! Generators take a [`Locale`](crux_analyzer_i18n::Locale) and render their
//! prose through [`Labels`]. Everything else they emit is Markdown/Mermaid
//! syntax or model data, which is locale-independent by contract.

use crux_analyzer_model::{Machine, Marker, State, StateDecl, Transition};

mod labels;
mod markdown;
mod mermaid;

pub use labels::Labels;
pub use markdown::markdown;
pub use mermaid::{mermaid_diagrams, Diagram};

/// Mermaid identifier for a state (or the wildcard pseudo-state). Composite
/// leaves (`Active/Loading`) become `Active_Loading`.
fn state_id(state: &str) -> String {
    if state == State::ANY {
        "any_state".to_string()
    } else {
        state.replace('/', "_")
    }
}

/// One `stateDiagram-v2` body for a machine (without code fences).
/// Composite leaves render nested inside their parent's block.
fn machine_diagram(machine: &Machine, labels: &Labels) -> String {
    let mut lines = vec!["stateDiagram-v2".to_string()];

    if machine
        .transitions
        .iter()
        .any(|t| t.from.0 == State::ANY || t.to.0 == State::ANY)
    {
        // Only the quoted label is localized: `any_state` is the node id the
        // transition lines refer to, so it must stay stable across locales.
        lines.push(format!(
            "    state \"{}\" as any_state",
            labels.any_state
        ));
    }

    // Composite blocks: `state Parent { state "Child" as Parent_Child }`.
    let mut seen_parents: Vec<&str> = Vec::new();
    for state in &machine.states {
        if let Some((parent, _)) = state.name.split_once('/') {
            if !seen_parents.contains(&parent) {
                seen_parents.push(parent);
                lines.push(format!("    state {parent} {{"));
                for leaf in &machine.states {
                    if let Some((p, child)) = leaf.name.split_once('/') {
                        if p == parent {
                            lines.push(format!(
                                "        state \"{child}\" as {}",
                                state_id(&leaf.name)
                            ));
                        }
                    }
                }
                lines.push("    }".to_string());
            }
        } else if !is_referenced(machine, &state.name) {
            // Orphan simple states still show up in the diagram.
            lines.push(format!("    {}", state_id(&state.name)));
        }
    }

    for transition in &machine.transitions {
        lines.push(format!(
            "    {} --> {}: {}",
            state_id(&transition.from.0),
            state_id(&transition.to.0),
            transition.event.0,
        ));
    }

    // Notes last: by now every id the diagram uses has been declared, whether
    // by a transition line, the orphan guard or a composite block — so this
    // needs no declaration line of its own and the orphan shortcut above
    // stays exactly as it is.
    for state in &machine.states {
        if let Some(doc) = &state.doc {
            lines.push(format!(
                "    note right of {} : {}",
                state_id(&state.name),
                note_text(doc),
            ));
        }
    }

    lines.join("\n")
}

/// A doc comment as a one-line Mermaid note.
///
/// The diagram is a hint — the states table below it carries the whole text —
/// so this takes the first paragraph and truncates on a word boundary. It is
/// the only place documentation is shortened.
fn note_text(doc: &str) -> String {
    let first = doc.split("\n\n").next().unwrap_or(doc);
    let flat = first.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= NOTE_MAX_CHARS {
        return flat;
    }
    let mut out: String = flat.chars().take(NOTE_MAX_CHARS).collect();
    if let Some(space) = out.rfind(' ') {
        out.truncate(space);
    }
    out.push('…');
    out
}

const NOTE_MAX_CHARS: usize = 72;

fn is_referenced(machine: &Machine, state: &str) -> bool {
    machine
        .transitions
        .iter()
        .any(|t| t.from.0 == state || t.to.0 == state)
}

/// The localized name of a marker.
///
/// An exhaustive match on purpose: a new [`Marker`] must not compile until
/// every locale has a word for it.
fn marker_label(marker: Marker, labels: &Labels) -> &'static str {
    match marker {
        Marker::Failure => labels.marker_failure,
        Marker::Deprecated => labels.marker_deprecated,
    }
}

/// Whether a machine has anything to put in a states table.
fn has_documented_states(machine: &Machine) -> bool {
    machine.states.iter().any(StateDecl::is_documented)
}

fn effects_cell(transition: &Transition, labels: &Labels) -> String {
    if transition.effects.is_empty() {
        labels.no_effects.to_string()
    } else {
        transition
            .effects
            .iter()
            .map(|e| format!("`{}`", e.0))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crux_analyzer_i18n::Locale;
    use crux_analyzer_model::{Core, Effect, Event, Project};

    fn sample() -> Project {
        Project {
            project: "Sample".into(),
            cores: vec![Core {
                name: "Player".into(),
                machines: vec![Machine {
                    name: "PlayerState".into(),
                    states: vec!["Stopped".into(), "Playing".into(), "Orphan".into()],
                    transitions: vec![
                        Transition {
                            from: State("Stopped".into()),
                            event: Event("Play".into()),
                            to: State("Playing".into()),
                            effects: vec![Effect("Render".into()), Effect("Audio::Start".into())],
                        },
                        Transition {
                            from: State(State::ANY.into()),
                            event: Event("Reset".into()),
                            to: State("Stopped".into()),
                            effects: vec![],
                        },
                    ],
                    ..Default::default()
                }],
            }],
        }
    }

    /// `sample()` with documentation on it. Kept separate so the undocumented
    /// project stays available as the backwards-compatibility guard.
    fn documented_sample() -> Project {
        let mut project = sample();
        let machine = &mut project.cores[0].machines[0];
        machine.doc = Some("Plays one track at a time.".into());
        machine.states[0] = StateDecl {
            name: "Stopped".into(),
            doc: Some("Nothing is playing.\n\nThe track stays loaded, so\nplay resumes it.".into()),
            markers: vec![],
            tags: vec!["idle-ish".into()],
        };
        machine.states[1] = StateDecl {
            name: "Playing".into(),
            doc: Some("Audio is reaching the speakers.".into()),
            markers: vec![Marker::Failure, Marker::Deprecated],
            tags: vec![],
        };
        project
    }

    #[test]
    fn mermaid_notes_document_states() {
        let body = &mermaid_diagrams(&documented_sample(), Locale::En)[0].mermaid;
        assert!(
            body.contains("note right of Playing : Audio is reaching the speakers."),
            "{body}"
        );
        // Undocumented states get no note.
        assert!(!body.contains("note right of Orphan"), "{body}");
        // Notes come after the transitions, so every id is declared by then.
        let first_transition = body.find("-->").expect("a transition line");
        let first_note = body.find("note right of").expect("a note line");
        assert!(first_note > first_transition, "{body}");
    }

    #[test]
    fn mermaid_notes_only_the_first_paragraph_on_one_line() {
        let body = &mermaid_diagrams(&documented_sample(), Locale::En)[0].mermaid;
        let note = body
            .lines()
            .find(|line| line.contains("note right of Stopped"))
            .expect("a note for Stopped");
        assert!(note.ends_with("Nothing is playing."), "{note}");
    }

    #[test]
    fn mermaid_truncates_a_long_state_doc() {
        let mut project = sample();
        project.cores[0].machines[0].states[1].doc =
            Some("word ".repeat(80).trim_end().to_string());
        let body = &mermaid_diagrams(&project, Locale::En)[0].mermaid;
        let note = body
            .lines()
            .find(|line| line.contains("note right of Playing"))
            .expect("a note");
        assert!(note.ends_with('…'), "{note}");
        assert!(note.chars().count() < 110, "{note}");
    }

    /// Documenting a state must not make it declare itself: the orphan
    /// shortcut is what keeps the diagram lean.
    #[test]
    fn mermaid_keeps_the_orphan_shortcut_when_states_are_documented() {
        let body = &mermaid_diagrams(&documented_sample(), Locale::En)[0].mermaid;
        assert!(!body.contains("state \"Playing\" as Playing"), "{body}");
        assert!(body.contains("\n    Orphan"), "{body}");
    }

    #[test]
    fn mermaid_notes_a_composite_child_by_its_flattened_id() {
        let mut project = Project {
            project: "S".into(),
            cores: vec![Core {
                name: "C".into(),
                machines: vec![Machine {
                    name: "State".into(),
                    states: vec!["Idle".into(), "Active/Loading".into()],
                    transitions: vec![Transition {
                        from: State("Idle".into()),
                        event: Event("Start".into()),
                        to: State("Active/Loading".into()),
                        effects: vec![],
                    }],
                    ..Default::default()
                }],
            }],
        };
        project.cores[0].machines[0].states[1].doc = Some("Fetching the manifest.".into());
        let body = &mermaid_diagrams(&project, Locale::En)[0].mermaid;
        assert!(
            body.contains("note right of Active_Loading : Fetching the manifest."),
            "{body}"
        );
    }

    /// Markers are words in a table, not colours in a diagram: a `classDef`
    /// fill would hardcode a hex that breaks in a dark-mode reader.
    #[test]
    fn mermaid_renders_no_marker_styling() {
        let body = &mermaid_diagrams(&documented_sample(), Locale::En)[0].mermaid;
        assert!(!body.contains("classDef"), "{body}");
        assert!(!body.contains("failure"), "{body}");
    }

    #[test]
    fn markdown_lists_documented_states() {
        let doc = markdown(&documented_sample(), Locale::En);
        assert!(doc.contains("#### States"), "{doc}");
        assert!(
            doc.contains("| State | Description | Markers | Tags |"),
            "{doc}"
        );
        assert!(
            doc.contains("| Playing | Audio is reaching the speakers. | failure, deprecated | — |"),
            "{doc}"
        );
        assert!(doc.contains("| Stopped | Nothing is playing. | — | `idle-ish` |"), "{doc}");
        // An undocumented state still has a row, so the table is the state list.
        assert!(doc.contains("| Orphan | — | — | — |"), "{doc}");
    }

    #[test]
    fn markdown_renders_the_machine_description_above_the_diagram() {
        let doc = markdown(&documented_sample(), Locale::En);
        let description = doc.find("Plays one track at a time.").expect("description");
        assert!(description < doc.find("```mermaid").unwrap(), "{doc}");
        assert!(description > doc.find("### Machine: PlayerState").unwrap(), "{doc}");
    }

    /// A cell is one line, so a longer description is repeated in full below
    /// the table rather than being truncated away.
    #[test]
    fn markdown_restates_a_multi_paragraph_description_in_full() {
        let doc = markdown(&documented_sample(), Locale::En);
        assert!(doc.contains("##### Stopped"), "{doc}");
        assert!(doc.contains("The track stays loaded, so\nplay resumes it."), "{doc}");
        // The single-paragraph state needs no section of its own.
        assert!(!doc.contains("##### Playing"), "{doc}");
    }

    /// A marker on the state enum describes the whole region, so it is stated
    /// beside the machine's description rather than in the per-state table.
    #[test]
    fn markdown_states_markers_declared_on_the_machine() {
        let mut project = sample();
        let machine = &mut project.cores[0].machines[0];
        machine.markers = vec![Marker::Deprecated];
        machine.tags = vec!["legacy".into()];

        let doc = markdown(&project, Locale::En);
        assert!(doc.contains("**Markers:** deprecated"), "{doc}");
        assert!(doc.contains("**Tags:** `legacy`"), "{doc}");
        // Region-level markers alone must not conjure a states table.
        assert!(!doc.contains("#### States"), "{doc}");

        let pt = markdown(&project, Locale::PtBr);
        assert!(pt.contains("**Marcadores:** descontinuado"), "{pt}");
        assert!(pt.contains("**Etiquetas:** `legacy`"), "{pt}");
    }

    #[test]
    fn markdown_omits_the_states_table_when_nothing_is_documented() {
        let doc = markdown(&sample(), Locale::En);
        assert!(!doc.contains("#### States"), "{doc}");
        assert!(!doc.contains("| State |"), "{doc}");
    }

    #[test]
    fn markdown_escapes_pipes_and_newlines_in_a_description() {
        let mut project = sample();
        project.cores[0].machines[0].states[1].doc =
            Some("Either a | or a\nwrapped line.".into());
        let doc = markdown(&project, Locale::En);
        assert!(doc.contains("| Playing | Either a \\| or a wrapped line. |"), "{doc}");
    }

    #[test]
    fn markdown_localizes_the_states_table_but_not_the_authors_text() {
        let doc = markdown(&documented_sample(), Locale::PtBr);
        assert!(doc.contains("#### Estados"), "{doc}");
        assert!(
            doc.contains("| Estado | Descrição | Marcadores | Etiquetas |"),
            "{doc}"
        );
        assert!(doc.contains("falha, descontinuado"), "{doc}");
        // The author's prose and their tag names are data, not prose of ours.
        assert!(doc.contains("Audio is reaching the speakers."), "{doc}");
        assert!(doc.contains("`idle-ish`"), "{doc}");
        assert!(!doc.contains("| State |"), "English label leaked: {doc}");
        assert!(!doc.contains("deprecated |"), "English marker leaked: {doc}");
    }

    /// The honesty rule as an assertion: switching locale must not touch a
    /// single character the analyzed application wrote.
    #[test]
    fn author_prose_is_identical_in_every_locale() {
        let project = documented_sample();
        for locale in Locale::ALL {
            let doc = markdown(&project, locale);
            assert!(doc.contains("Plays one track at a time."), "{locale}");
            assert!(doc.contains("Audio is reaching the speakers."), "{locale}");
            assert!(
                doc.contains("The track stays loaded, so\nplay resumes it."),
                "{locale}"
            );
        }
    }

    #[test]
    fn mermaid_renders_composite_states_nested() {
        let project = Project {
            project: "S".into(),
            cores: vec![Core {
                name: "C".into(),
                machines: vec![Machine {
                    name: "State".into(),
                    states: vec![
                        "Idle".into(),
                        "Active/Loading".into(),
                        "Active/Ready".into(),
                    ],
                    transitions: vec![Transition {
                        from: State("Idle".into()),
                        event: Event("Start".into()),
                        to: State("Active/Loading".into()),
                        effects: vec![],
                    }],
                    ..Default::default()
                }],
            }],
        };
        let body = &mermaid_diagrams(&project, Locale::En)[0].mermaid;
        assert!(body.contains("state Active {"), "{body}");
        assert!(body.contains("state \"Loading\" as Active_Loading"), "{body}");
        assert!(body.contains("Idle --> Active_Loading: Start"), "{body}");
    }

    #[test]
    fn mermaid_renders_transitions_wildcards_and_orphans() {
        let diagrams = mermaid_diagrams(&sample(), Locale::En);
        assert_eq!(diagrams.len(), 1);
        assert_eq!(diagrams[0].core, "Player");
        assert_eq!(diagrams[0].machine, "PlayerState");

        let body = &diagrams[0].mermaid;
        assert!(body.starts_with("stateDiagram-v2"), "{body}");
        assert!(body.contains("Stopped --> Playing: Play"), "{body}");
        assert!(body.contains("state \"any state\" as any_state"), "{body}");
        assert!(body.contains("any_state --> Stopped: Reset"), "{body}");
        assert!(body.contains("\n    Orphan"), "orphan state must appear: {body}");
    }

    #[test]
    fn markdown_embeds_diagrams_and_transition_tables() {
        let doc = markdown(&sample(), Locale::En);
        assert!(doc.contains("# Sample"), "{doc}");
        assert!(doc.contains("## Core: Player"), "{doc}");
        assert!(doc.contains("### Machine: PlayerState"), "{doc}");
        assert!(doc.contains("```mermaid\nstateDiagram-v2"), "{doc}");
        assert!(doc.contains("| Stopped | `Play` | Playing | `Render`, `Audio::Start` |"), "{doc}");
        assert!(doc.contains("| *any* | `Reset` | Stopped | — |"), "{doc}");
    }

    #[test]
    fn markdown_localizes_prose_and_leaves_identifiers_alone() {
        let doc = markdown(&sample(), Locale::PtBr);
        assert!(doc.contains("## Núcleo: Player"), "{doc}");
        assert!(doc.contains("### Máquina: PlayerState"), "{doc}");
        assert!(doc.contains("| De | Evento | Para | Efeitos |"), "{doc}");
        assert!(doc.contains("| *qualquer* | `Reset` | Stopped | — |"), "{doc}");
        // The project title and every identifier are data, not prose.
        assert!(doc.contains("# Sample"), "{doc}");
        assert!(
            doc.contains("| Stopped | `Play` | Playing | `Render`, `Audio::Start` |"),
            "{doc}"
        );
        assert!(!doc.contains("## Core:"), "English label leaked: {doc}");
    }

    #[test]
    fn mermaid_localizes_the_wildcard_label_but_not_its_node_id() {
        let body = &mermaid_diagrams(&sample(), Locale::PtBr)[0].mermaid;
        assert!(body.contains("state \"qualquer estado\" as any_state"), "{body}");
        // The id is diagram syntax: the transition line must still resolve.
        assert!(body.contains("any_state --> Stopped: Reset"), "{body}");
        assert!(body.starts_with("stateDiagram-v2"), "{body}");
    }
}
