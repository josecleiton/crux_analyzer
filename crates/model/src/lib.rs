//! Semantic structs of the crux_analyzer intermediate model.
//!
//! This crate contains data only: no parsing logic (that belongs to
//! `crux-analyzer-parser`) and no UI logic. Serialization follows the
//! contract defined in `shared/schema/crux-model.schema.json`.

use serde::{Deserialize, Serialize};

/// Analyzed project — root of the model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub project: String,
    pub cores: Vec<Core>,
}

/// A Core (Crux app) identified in the source code.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Core {
    pub name: String,
    /// State machines (orthogonal regions) of this Core, statechart style —
    /// one per state enum found in the Core's model.
    pub machines: Vec<Machine>,
}

/// One state machine of a Core.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Machine {
    /// Usually the state enum's name (e.g. `RecordingState`).
    pub name: String,
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
}

/// A state of a machine.
///
/// In the serialized contract it is just the name (a string); extra
/// fields (docs, code spans) will come in future schema versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct State(pub String);

impl State {
    /// The wildcard source state: a transition that fires from any state.
    pub const ANY: &'static str = "*";
}

/// An event that triggers transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Event(pub String);

/// An effect requested by the Core through a capability
/// (e.g. `Render`, `AudioOperation::Start`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Effect(pub String);

/// A capability used by the Core (Http, KeyValue, ...).
/// Not part of the MVP serialized contract yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Capability(pub String);

/// State transition triggered by an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transition {
    pub from: State,
    pub event: Event,
    pub to: State,
    /// Effects requested when this transition fires.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<Effect>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The model must round-trip with the contract example
    /// (`shared/schema/examples/audio-recorder.json`).
    #[test]
    fn roundtrips_schema_example() {
        let json = include_str!("../../../shared/schema/examples/audio-recorder.json");
        let project: Project = serde_json::from_str(json).expect("example must deserialize");

        assert_eq!(project.project, "Audio Recorder");
        assert_eq!(project.cores.len(), 3);

        let recorder = &project.cores[0];
        assert_eq!(recorder.name, "Recorder");
        assert_eq!(recorder.machines.len(), 2);

        let machine = &recorder.machines[0];
        assert_eq!(machine.name, "RecorderState");
        assert_eq!(machine.states.len(), 5);
        assert_eq!(machine.transitions.len(), 5);
        assert_eq!(
            machine.transitions[0],
            Transition {
                from: State("Idle".into()),
                event: Event("RecordPressed".into()),
                to: State("Recording".into()),
                effects: vec![Effect("AudioOperation::Start".into())],
            }
        );

        // Wildcard source state round-trips untouched.
        let inputs = &recorder.machines[1];
        assert_eq!(inputs.transitions[2].from.0, State::ANY);
        // Absent `effects` deserializes as empty and is skipped when empty.
        assert!(inputs.transitions[2].effects.is_empty());

        let reserialized = serde_json::to_string(&project).expect("must serialize");
        assert!(!reserialized.contains("\"effects\":[]"));
        let reparsed: Project = serde_json::from_str(&reserialized).unwrap();
        assert_eq!(project, reparsed);
    }
}
