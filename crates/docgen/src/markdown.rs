//! Markdown generator: one document with a Mermaid diagram and a transition
//! table per machine.

use crux_analyzer_model::{Project, State};

use crate::{effects_cell, machine_diagram};

pub fn markdown(project: &Project) -> String {
    let mut out = String::new();
    push_line(&mut out, &format!("# {}", project.project));

    for core in &project.cores {
        push_line(&mut out, "");
        push_line(&mut out, &format!("## Core: {}", core.name));

        for machine in &core.machines {
            push_line(&mut out, "");
            push_line(&mut out, &format!("### Machine: {}", machine.name));
            push_line(&mut out, "");
            push_line(&mut out, "```mermaid");
            push_line(&mut out, &machine_diagram(machine));
            push_line(&mut out, "```");
            push_line(&mut out, "");
            push_line(&mut out, "| From | Event | To | Effects |");
            push_line(&mut out, "| --- | --- | --- | --- |");
            for transition in &machine.transitions {
                let from = if transition.from.0 == State::ANY {
                    "*any*".to_string()
                } else {
                    transition.from.0.clone()
                };
                push_line(
                    &mut out,
                    &format!(
                        "| {from} | `{}` | {} | {} |",
                        transition.event.0,
                        transition.to.0,
                        effects_cell(transition),
                    ),
                );
            }
        }
    }

    out
}

fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}
