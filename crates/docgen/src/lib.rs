//! Documentation generators for the crux_analyzer semantic model.
//!
//! Every generator consumes only [`crux_analyzer_model`] types — never the
//! parser or its AST — so they work for any client that has a model JSON.
//!
//! Generators take a [`Locale`](crux_analyzer_i18n::Locale) and render their
//! prose through [`Labels`]. Everything else they emit is Markdown/Mermaid
//! syntax or model data, which is locale-independent by contract.

use crux_analyzer_model::{Machine, State, Transition};

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
    lines.join("\n")
}

fn is_referenced(machine: &Machine, state: &str) -> bool {
    machine
        .transitions
        .iter()
        .any(|t| t.from.0 == state || t.to.0 == state)
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
