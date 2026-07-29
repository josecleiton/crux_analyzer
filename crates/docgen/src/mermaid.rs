//! Mermaid `stateDiagram-v2` generator.

use crux_analyzer_i18n::Locale;
use crux_analyzer_model::Project;

use crate::{machine_diagram, Labels};

/// One Mermaid diagram per state machine.
pub struct Diagram {
    pub core: String,
    pub machine: String,
    /// `stateDiagram-v2` source, without code fences.
    pub mermaid: String,
}

pub fn mermaid_diagrams(project: &Project, locale: Locale) -> Vec<Diagram> {
    let labels = Labels::for_locale(locale);
    project
        .cores
        .iter()
        .flat_map(|core| {
            core.machines.iter().map(|machine| Diagram {
                core: core.name.clone(),
                machine: machine.name.clone(),
                mermaid: machine_diagram(machine, &labels),
            })
        })
        .collect()
}
