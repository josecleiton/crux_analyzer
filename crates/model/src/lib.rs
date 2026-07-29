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
    pub states: Vec<State>,
    pub transitions: Vec<Transition>,
}

/// A state of the Core's model.
///
/// In the serialized contract it is just the name (a string); extra
/// fields (docs, code spans) will come in future schema versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct State(pub String);

/// An event that triggers transitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Event(pub String);

/// An effect requested by the Core through a capability.
/// Not part of the MVP serialized contract yet.
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
        assert_eq!(recorder.states.len(), 5);
        assert_eq!(recorder.transitions.len(), 5);
        assert_eq!(
            recorder.transitions[0],
            Transition {
                from: State("Idle".into()),
                event: Event("RecordPressed".into()),
                to: State("Recording".into()),
            }
        );

        let reserialized = serde_json::to_string(&project).expect("must serialize");
        let reparsed: Project = serde_json::from_str(&reserialized).unwrap();
        assert_eq!(project, reparsed);
    }
}
