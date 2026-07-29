//! Semantic structs of the crux_analyzer intermediate model.
//!
//! This crate contains data only: no parsing logic (that belongs to
//! `crux-analyzer-parser`) and no UI logic. Serialization follows the
//! contract defined in `shared/schema/crux-model.schema.json`.
//!
//! # Text in the model
//!
//! Everything here is read out of the analyzed application: identifiers
//! (core, machine, state, event and effect names), the author's own prose
//! ([`StateDecl::doc`], [`Machine::doc`]) and the author's tag names. None of
//! it is ever translated — the model is locale-independent by contract. Only
//! [`Marker`] is crux_analyzer's own vocabulary, which is why it travels as a
//! stable identifier and gets its human labels from the clients.

use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Machine {
    /// Usually the state enum's name (e.g. `RecordingState`).
    pub name: String,
    /// Documentation authored on the state enum itself, annotation lines
    /// removed. Absent when the source carries none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// Markers declared on the state enum — they describe the whole region.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<Marker>,
    /// Free-form tag names declared on the state enum.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub states: Vec<StateDecl>,
    pub transitions: Vec<Transition>,
}

/// A semantic marker declared in the analyzed source, as an `@` line inside a
/// `///` doc comment.
///
/// The wire names (`failure`, `deprecated`) are stable identifiers, not prose:
/// tooling matches them and they are never translated. Human labels live in
/// `crux_analyzer_docgen::Labels` and in the web catalogs.
///
/// Deliberately a **closed** vocabulary — adding a variant would make an older
/// build of this crate fail to deserialize a newer model. [`StateDecl::tags`]
/// is the open-ended escape hatch, so reach for a tag before a variant.
///
/// `initial` and `final` are absent on purpose: they are derived from graph
/// shape (and `#[default]`), so declaring them would invite a source to
/// contradict the transitions it also declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Marker {
    /// The state stands for a failure the application recognizes as such.
    Failure,
    /// The state is kept for compatibility and is on its way out.
    Deprecated,
}

impl Marker {
    /// Stable, locale-independent identifier — the counterpart of
    /// `WarningKind::code()`. Key tooling and documentation on this, never on
    /// a rendered label.
    pub fn code(self) -> &'static str {
        match self {
            Marker::Failure => "failure",
            Marker::Deprecated => "deprecated",
        }
    }
}

/// A state as **declared** by a machine: its name plus whatever the analyzed
/// source documents about it.
///
/// This is the counterpart of [`State`], which is a *reference* to a
/// declaration (or the wildcard). Only declarations carry documentation, which
/// is what keeps [`Transition`] comparable by value — see the dedup in the
/// parser's `emit`.
///
/// # Serialization
///
/// Serializes as a **bare string** when it carries no documentation, and as an
/// object otherwise; both forms deserialize. An app without doc comments
/// therefore produces exactly the JSON it produced before this type existed.
///
/// Two deliberate asymmetries with the JSON Schema: the schema sets
/// `additionalProperties: false`, but this reader is lenient and **ignores**
/// unknown object fields, so a newer model never breaks an older build; and
/// because the object form is read through an untagged enum, the error for a
/// malformed object is generic rather than field-specific.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StateDecl {
    /// Leaf state name. `Parent/Child` for a composite state's child.
    pub name: String,
    /// Documentation authored on the enum variant, annotation lines removed.
    pub doc: Option<String>,
    /// Declared markers, in first-seen order, deduplicated.
    pub markers: Vec<Marker>,
    /// Free-form `@tag` names, in first-seen order, deduplicated. Identifiers
    /// from the analyzed app — never translated.
    pub tags: Vec<String>,
}

impl StateDecl {
    /// A state with a name and no documentation — the form that serializes as
    /// a bare string.
    pub fn bare(name: impl Into<String>) -> Self {
        StateDecl {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Whether this declaration is nothing but a name.
    pub fn is_bare(&self) -> bool {
        self.doc.is_none() && self.markers.is_empty() && self.tags.is_empty()
    }

    /// Whether the source documented this state in any way.
    pub fn is_documented(&self) -> bool {
        !self.is_bare()
    }

    pub fn has_marker(&self, marker: Marker) -> bool {
        self.markers.contains(&marker)
    }
}

impl From<&str> for StateDecl {
    fn from(name: &str) -> Self {
        StateDecl::bare(name)
    }
}

impl From<String> for StateDecl {
    fn from(name: String) -> Self {
        StateDecl::bare(name)
    }
}

impl Serialize for StateDecl {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.is_bare() {
            return serializer.serialize_str(&self.name);
        }
        let fields = 1
            + usize::from(self.doc.is_some())
            + usize::from(!self.markers.is_empty())
            + usize::from(!self.tags.is_empty());
        let mut state = serializer.serialize_struct("StateDecl", fields)?;
        state.serialize_field("name", &self.name)?;
        if let Some(doc) = &self.doc {
            state.serialize_field("doc", doc)?;
        }
        if !self.markers.is_empty() {
            state.serialize_field("markers", &self.markers)?;
        }
        if !self.tags.is_empty() {
            state.serialize_field("tags", &self.tags)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for StateDecl {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        /// Both authored forms of a state, collapsed on the way in.
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Name(String),
            Full {
                name: String,
                #[serde(default)]
                doc: Option<String>,
                #[serde(default)]
                markers: Vec<Marker>,
                #[serde(default)]
                tags: Vec<String>,
            },
        }

        Ok(match Repr::deserialize(deserializer)? {
            Repr::Name(name) => StateDecl::bare(name),
            Repr::Full {
                name,
                doc,
                markers,
                tags,
            } => StateDecl {
                name,
                doc,
                markers,
                tags,
            },
        })
    }
}

/// A **reference** to a state of a machine: the name of a [`StateDecl`], or
/// [`State::ANY`].
///
/// Stays a bare string in the serialized contract, and stays comparable by
/// value — [`Transition`] is deduplicated by equality, so an endpoint must
/// never carry metadata that could make two otherwise-identical transitions
/// differ. Documentation lives on the declaration, not on the reference.
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

    /// Both authored forms of a state coexist in the bundled example, and the
    /// undocumented ones stay bare strings through a round trip.
    #[test]
    fn reads_both_state_forms_from_the_schema_example() {
        let json = include_str!("../../../shared/schema/examples/audio-recorder.json");
        let project: Project = serde_json::from_str(json).unwrap();

        // RecorderState is entirely undocumented: the pre-documentation shape.
        let recorder = &project.cores[0].machines[0];
        assert!(recorder.doc.is_none());
        assert!(recorder.states.iter().all(StateDecl::is_bare));

        // AuthState mixes bare and annotated states.
        let auth = &project.cores[1].machines[0];
        let failed = auth.states.iter().find(|s| s.name == "Failed").unwrap();
        assert!(failed.has_marker(Marker::Failure));
        assert_eq!(failed.tags, ["retryable"]);
        assert!(failed.doc.as_deref().unwrap().contains("refused"));
        assert!(auth.states[0].is_bare(), "SignedOut must stay bare");

        // SyncState documents the machine itself.
        let sync = &project.cores[2].machines[0];
        assert!(sync.doc.as_deref().unwrap().contains("device"));
        let done = sync.states.iter().find(|s| s.name == "Done").unwrap();
        assert_eq!(done.markers, [Marker::Deprecated]);
        assert!(done.doc.is_none(), "a marker alone needs no prose");

        // Undocumented states still serialize bare, documented ones as objects.
        let reserialized = serde_json::to_string(&project).unwrap();
        assert!(reserialized.contains(r#""states":["Idle","Recording""#));
        assert!(reserialized.contains(r#""markers":["failure"]"#));
    }

    #[test]
    fn undocumented_state_declarations_serialize_as_bare_strings() {
        let bare = StateDecl::bare("Idle");
        assert_eq!(serde_json::to_string(&bare).unwrap(), r#""Idle""#);

        let documented = StateDecl {
            name: "Failed".into(),
            doc: Some("It broke.".into()),
            markers: vec![Marker::Failure],
            tags: vec!["retryable".into()],
        };
        assert_eq!(
            serde_json::to_string(&documented).unwrap(),
            r#"{"name":"Failed","doc":"It broke.","markers":["failure"],"tags":["retryable"]}"#
        );
    }

    /// An object carrying nothing but a name canonicalizes back to the bare
    /// form, so a hand-written model and a generated one compare equal.
    #[test]
    fn a_name_only_object_canonicalizes_to_the_bare_form() {
        let decl: StateDecl = serde_json::from_str(r#"{"name":"Idle"}"#).unwrap();
        assert_eq!(decl, StateDecl::bare("Idle"));
        assert_eq!(serde_json::to_string(&decl).unwrap(), r#""Idle""#);
    }

    #[test]
    fn deserializes_a_mixed_state_array() {
        let states: Vec<StateDecl> = serde_json::from_str(
            r#"["Idle", {"name":"Failed","markers":["failure"]}, {"name":"Old","tags":["legacy"]}]"#,
        )
        .unwrap();
        assert_eq!(
            states.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
            ["Idle", "Failed", "Old"]
        );
        assert!(states[0].is_bare());
        assert_eq!(states[1].markers, [Marker::Failure]);
        assert_eq!(states[2].tags, ["legacy"]);
    }

    /// Marker wire names are the stable identifiers tooling keys on.
    #[test]
    fn marker_wire_names_are_stable() {
        assert_eq!(serde_json::to_string(&Marker::Failure).unwrap(), r#""failure""#);
        assert_eq!(
            serde_json::to_string(&Marker::Deprecated).unwrap(),
            r#""deprecated""#
        );
        assert_eq!(Marker::Failure.code(), "failure");
        assert_eq!(Marker::Deprecated.code(), "deprecated");
    }

    /// An undocumented machine emits exactly the keys it emitted before
    /// documentation existed.
    #[test]
    fn an_undocumented_machine_emits_no_documentation_keys() {
        let machine = Machine {
            name: "UploadState".into(),
            states: vec!["Empty".into(), "Synced".into()],
            ..Default::default()
        };
        assert_eq!(
            serde_json::to_string(&machine).unwrap(),
            r#"{"name":"UploadState","states":["Empty","Synced"],"transitions":[]}"#
        );
    }
}
