//! Documentation generators for the crux_analyzer semantic model.
//!
//! Every generator consumes only [`crux_analyzer_model`] types — never the
//! parser or its AST — so they work for any client that has a model JSON.

use crux_analyzer_model::{Machine, State, Transition};

mod markdown;
mod mermaid;

pub use markdown::markdown;
pub use mermaid::{mermaid_diagrams, Diagram};

/// Mermaid identifier for a state (or the wildcard pseudo-state).
fn state_id(state: &str) -> String {
    if state == State::ANY {
        "any_state".to_string()
    } else {
        state.to_string()
    }
}

/// One `stateDiagram-v2` body for a machine (without code fences).
fn machine_diagram(machine: &Machine) -> String {
    let mut lines = vec!["stateDiagram-v2".to_string()];

    if machine.transitions.iter().any(|t| t.from.0 == State::ANY) {
        lines.push("    state \"any state\" as any_state".to_string());
    }
    for state in &machine.states {
        if !is_referenced(machine, &state.0) {
            // Orphan states still show up in the diagram.
            lines.push(format!("    {}", state_id(&state.0)));
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

fn effects_cell(transition: &Transition) -> String {
    if transition.effects.is_empty() {
        "—".to_string()
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
    use crux_analyzer_model::{Core, Effect, Event, Project};

    fn sample() -> Project {
        Project {
            project: "Sample".into(),
            cores: vec![Core {
                name: "Player".into(),
                machines: vec![Machine {
                    name: "PlayerState".into(),
                    states: vec![
                        State("Stopped".into()),
                        State("Playing".into()),
                        State("Orphan".into()),
                    ],
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
                }],
            }],
        }
    }

    #[test]
    fn mermaid_renders_transitions_wildcards_and_orphans() {
        let diagrams = mermaid_diagrams(&sample());
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
        let doc = markdown(&sample());
        assert!(doc.contains("# Sample"), "{doc}");
        assert!(doc.contains("## Core: Player"), "{doc}");
        assert!(doc.contains("### Machine: PlayerState"), "{doc}");
        assert!(doc.contains("```mermaid\nstateDiagram-v2"), "{doc}");
        assert!(doc.contains("| Stopped | `Play` | Playing | `Render`, `Audio::Start` |"), "{doc}");
        assert!(doc.contains("| *any* | `Reset` | Stopped | — |"), "{doc}");
    }
}
