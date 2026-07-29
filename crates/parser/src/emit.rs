//! Assembles the extracted data into the semantic model.

use std::collections::BTreeMap;

use crux_analyzer_model::{Core, Effect, Event, Machine, State, Transition};

use crate::core_finder::CoreInfo;
use crate::state_enum::StateMachine;
use crate::transitions::RawTransition;

/// Builds a [`Core`] from a core's extraction result: one [`Machine`] per
/// state enum that contributed transitions (orthogonal regions).
pub(crate) fn to_core(core: &CoreInfo, machines: &[StateMachine], raw: Vec<RawTransition>) -> Core {
    // Keyed by (enum, field): the same enum can drive several machines
    // through different fields, and each keeps its own transitions.
    let mut by_machine: BTreeMap<(String, String), Vec<RawTransition>> = BTreeMap::new();
    for transition in raw {
        by_machine
            .entry((transition.machine.clone(), transition.field.clone()))
            .or_default()
            .push(transition);
    }

    let model_machines = machines
        .iter()
        .filter_map(|machine| {
            let raw_transitions =
                by_machine.remove(&(machine.enum_name.clone(), machine.field_name.clone()))?;

            let mut transitions: Vec<Transition> = Vec::new();
            for raw in raw_transitions {
                let transition = Transition {
                    from: State(raw.from),
                    event: Event(raw.event),
                    to: State(raw.to),
                    effects: raw.effects.into_iter().map(Effect).collect(),
                };
                if !transitions.contains(&transition) {
                    transitions.push(transition);
                }
            }

            Some(Machine {
                name: machine_name(machine, machines),
                states: machine.variants.iter().map(|v| State(v.clone())).collect(),
                transitions,
            })
        })
        .collect();

    Core {
        name: core.name.clone(),
        machines: model_machines,
    }
}

/// The enum name, disambiguated by field when the same enum drives more than
/// one machine (e.g. two fields of the same state enum).
fn machine_name(machine: &StateMachine, machines: &[StateMachine]) -> String {
    let same_enum = machines.iter().filter(|m| m.enum_name == machine.enum_name).count();
    if same_enum > 1 {
        format!("{} ({})", machine.enum_name, machine.field_name)
    } else {
        machine.enum_name.clone()
    }
}
