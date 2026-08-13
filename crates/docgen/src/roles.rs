//! Derived state roles: what a machine's *shape* says about a state, as opposed
//! to what its author declared.
//!
//! A [`Marker`](crux_analyzer_model::Marker) is declared; `initial` and `final`
//! are concluded — which is why they are not markers, and why the conclusion
//! lives in the clients rather than in the model. This module is the generators'
//! copy of that reading, and it is deliberately the same reading the web UI
//! makes in `apps/web/src/domain/stateRole.ts`: the diagram, the states table
//! and the canvas must not disagree about where a machine starts and where it
//! ends. Change one, change both.

use std::collections::HashSet;

use crux_analyzer_model::{Machine, State};

/// The `initial` and `final` roles of one machine's states, computed once.
///
/// # Initial
///
/// The machine's entry point, in order of evidence:
///
/// 1. the state the source declares as its enum's `#[default]` variant;
/// 2. otherwise every state nothing transitions into;
/// 3. otherwise — a fully cyclic machine, where neither kind of evidence
///    exists — the first declared state.
///
/// Step 2 can hold for several states at once (an orphan region has an entry of
/// its own), and all of them are entry points; step 1 collapses to exactly one.
///
/// # Final
///
/// A dead end: no outgoing transition of its own. A machine-wide wildcard
/// (`from: "*"`) may still leave it, but that escape belongs to the "any state"
/// pseudo-node — counting it would erase every final state of every machine
/// that has one.
pub struct MachineRoles<'a> {
    initial: HashSet<&'a str>,
    finals: HashSet<&'a str>,
}

impl<'a> MachineRoles<'a> {
    pub fn of(machine: &'a Machine) -> Self {
        let targets: HashSet<&str> = machine
            .transitions
            .iter()
            .map(|transition| transition.to.0.as_str())
            .collect();
        let sources: HashSet<&str> = machine
            .transitions
            .iter()
            .map(|transition| transition.from.0.as_str())
            .collect();

        let names = || machine.states.iter().map(|state| state.name.as_str());
        let declared = machine
            .states
            .iter()
            .find(|state| state.is_default)
            .map(|state| state.name.as_str());
        let initial: HashSet<&str> = match declared {
            Some(default) => HashSet::from([default]),
            None => {
                let entry_points: HashSet<&str> =
                    names().filter(|name| !targets.contains(name)).collect();
                if entry_points.is_empty() {
                    names().next().into_iter().collect()
                } else {
                    entry_points
                }
            }
        };

        Self {
            initial,
            finals: names()
                .filter(|name| !sources.contains(name) && *name != State::ANY)
                .collect(),
        }
    }

    pub fn is_initial(&self, state: &str) -> bool {
        self.initial.contains(state)
    }

    pub fn is_final(&self, state: &str) -> bool {
        self.finals.contains(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crux_analyzer_model::{Event, StateDecl, Transition};

    fn machine(states: Vec<StateDecl>, transitions: &[(&str, &str, &str)]) -> Machine {
        Machine {
            name: "M".into(),
            states,
            transitions: transitions
                .iter()
                .map(|(from, event, to)| Transition {
                    from: State(from.to_string()),
                    event: Event(event.to_string()),
                    to: State(to.to_string()),
                    effects: Vec::new(),
                })
                .collect(),
            ..Default::default()
        }
    }

    fn default_state(name: &str) -> StateDecl {
        StateDecl {
            name: name.into(),
            is_default: true,
            ..Default::default()
        }
    }

    #[test]
    fn a_state_nothing_arrives_at_is_the_entry_point() {
        let machine = machine(
            vec!["Idle".into(), "Running".into(), "Done".into()],
            &[("Idle", "Go", "Running"), ("Running", "Stop", "Done")],
        );
        let roles = MachineRoles::of(&machine);
        assert!(roles.is_initial("Idle"));
        assert!(!roles.is_initial("Running"));
        assert!(roles.is_final("Done"));
        assert!(!roles.is_final("Running"));
    }

    /// The gap the `#[default]` evidence closes: a cycle gives declaration order
    /// no meaning, so without the declaration the entry point is a guess.
    #[test]
    fn a_cyclic_machine_takes_its_entry_point_from_the_declaration() {
        let states = vec!["Idle".into(), default_state("Running"), "Done".into()];
        let cycle = &[
            ("Idle", "Go", "Running"),
            ("Running", "Stop", "Done"),
            ("Done", "Reset", "Idle"),
        ];
        let declared = machine(states, cycle);
        let roles = MachineRoles::of(&declared);
        assert!(roles.is_initial("Running"));
        assert!(
            !roles.is_initial("Idle"),
            "declaration order is not evidence"
        );

        // Same machine, no declaration: the first state stands in, which is all
        // the shape offers.
        let undeclared = machine(vec!["Idle".into(), "Running".into(), "Done".into()], cycle);
        let roles = MachineRoles::of(&undeclared);
        assert!(roles.is_initial("Idle"));
        assert!(!roles.is_initial("Running"));
    }

    /// A declaration outranks the shape even when they disagree: the source is
    /// entitled to start a machine somewhere transitions also lead back to.
    #[test]
    fn the_declaration_outranks_an_unreachable_state() {
        let machine = machine(
            vec!["Orphan".into(), default_state("Idle")],
            &[("Idle", "Go", "Idle")],
        );
        let roles = MachineRoles::of(&machine);
        assert!(roles.is_initial("Idle"));
        assert!(!roles.is_initial("Orphan"));
        assert!(roles.is_final("Orphan"), "nothing leaves it");
    }

    /// Every state of a machine whose only transitions are wildcards is both an
    /// entry point and a dead end — the wildcard escape belongs to the pseudo
    /// node, not to the states it stands for.
    #[test]
    fn a_wildcard_transition_is_not_a_state_of_its_own() {
        let machine = machine(
            vec!["Idle".into(), "Failed".into()],
            &[("*", "Panic", "Failed")],
        );
        let roles = MachineRoles::of(&machine);
        assert!(roles.is_initial("Idle"));
        assert!(!roles.is_initial("Failed"), "a transition arrives here");
        assert!(roles.is_final("Idle") && roles.is_final("Failed"));
        assert!(!roles.is_final(State::ANY), "the pseudo node has no role");
    }
}
