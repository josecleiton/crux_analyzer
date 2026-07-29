//! Mermaid `stateDiagram-v2` generator.

use crux_analyzer_model::Project;

use crate::machine_diagram;

/// One Mermaid diagram per state machine.
pub struct Diagram {
    pub core: String,
    pub machine: String,
    /// `stateDiagram-v2` source, without code fences.
    pub mermaid: String,
}

pub fn mermaid_diagrams(project: &Project) -> Vec<Diagram> {
    project
        .cores
        .iter()
        .flat_map(|core| {
            core.machines.iter().map(|machine| Diagram {
                core: core.name.clone(),
                machine: machine.name.clone(),
                mermaid: machine_diagram(machine),
            })
        })
        .collect()
}
