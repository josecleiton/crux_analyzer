//! Assembles the extracted data into the semantic model.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crux_analyzer_model::{Core, Event, State, Transition};

use crate::core_finder::CoreInfo;
use crate::state_enum::StateMachine;
use crate::transitions::RawTransition;
use crate::Warning;

/// Builds a [`Core`] from a core's extraction result.
///
/// When more than one state machine contributed transitions, the one with the
/// most transitions wins and the others are reported as warnings (the schema
/// currently models a single state list per core).
pub(crate) fn to_core(
    core: &CoreInfo,
    machines: &[StateMachine],
    raw: Vec<RawTransition>,
    warnings: &mut Vec<Warning>,
) -> Core {
    let mut by_machine: BTreeMap<String, Vec<RawTransition>> = BTreeMap::new();
    for transition in raw {
        by_machine
            .entry(transition.machine.clone())
            .or_default()
            .push(transition);
    }

    let chosen = by_machine
        .iter()
        .max_by_key(|(_, transitions)| transitions.len())
        .map(|(machine, _)| machine.clone());

    for machine in by_machine.keys() {
        if Some(machine) != chosen.as_ref() {
            warnings.push(Warning {
                file: PathBuf::new(),
                line: 0,
                message: format!(
                    "core {}: state machine `{machine}` also has transitions; \
                     only `{}` was emitted (multi-machine cores are future work)",
                    core.name,
                    chosen.as_deref().unwrap_or("?"),
                ),
            });
        }
    }

    let states = chosen
        .as_ref()
        .and_then(|machine| machines.iter().find(|m| m.enum_name == *machine))
        .map(|m| m.variants.iter().map(|v| State(v.clone())).collect())
        .unwrap_or_default();

    let mut transitions: Vec<Transition> = Vec::new();
    if let Some(machine) = &chosen {
        for raw in &by_machine[machine] {
            let transition = Transition {
                from: State(raw.from.clone()),
                event: Event(raw.event.clone()),
                to: State(raw.to.clone()),
            };
            if !transitions.contains(&transition) {
                transitions.push(transition);
            }
        }
    }

    Core {
        name: core.name.clone(),
        states,
        transitions,
    }
}
