//! Assembles the extracted data into the semantic model.

use std::collections::{BTreeMap, BTreeSet};

use crux_analyzer_model::{
    Core, DocumentedName, Effect, Event, Machine, State, StateDecl, Transition,
};

use crate::annotations::DocBlock;
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

    let model_machines: Vec<Machine> = machines
        .iter()
        .filter_map(|machine| {
            let raw_transitions =
                by_machine.remove(&(machine.enum_name.clone(), machine.field_name.clone()))?;

            // Deduplicated through a set, not a linear scan: a wide guard can
            // fan one assignment out to variants × events, and comparing every
            // new transition against every kept one is quadratic in that.
            let mut transitions: Vec<Transition> = Vec::new();
            let mut seen: std::collections::HashSet<(String, String, String, Vec<String>)> =
                std::collections::HashSet::new();
            for raw in raw_transitions {
                if !seen.insert((
                    raw.from.clone(),
                    raw.event.clone(),
                    raw.to.clone(),
                    raw.effects.clone(),
                )) {
                    continue;
                }
                transitions.push(Transition {
                    from: State(raw.from),
                    event: Event(raw.event),
                    to: State(raw.to),
                    effects: raw.effects.into_iter().map(Effect).collect(),
                });
            }

            Some(Machine {
                name: machine_name(machine, machines),
                doc: machine.docs.doc.clone(),
                markers: machine.docs.markers.clone(),
                tags: machine.docs.tags.clone(),
                states: machine
                    .variants
                    .iter()
                    .zip(&machine.variant_docs)
                    .map(|(name, docs)| state_decl(name, docs))
                    .collect(),
                transitions,
            })
        })
        .collect();

    let events = documented_events(core, &model_machines);
    let effects = documented_effects(core, &model_machines);
    Core {
        name: core.name.clone(),
        machines: model_machines,
        events,
        effects,
    }
}

/// Documentation authored on event enum variants, for the events this core's
/// transitions actually use. Reading what the source *declares* — the honesty
/// rule's fair game. Restricted to used events so the catalog joins cleanly
/// with the transition tables: a documented wrapper variant (a delegating
/// `Event::Recording(RecordingEvent)`) never appears as a phantom event.
fn documented_events(core: &CoreInfo, machines: &[Machine]) -> Vec<DocumentedName> {
    let used: BTreeSet<&str> = machines
        .iter()
        .flat_map(|machine| &machine.transitions)
        .map(|transition| transition.event.0.as_str())
        .collect();

    // BTreeMap: deterministic output and one entry per name — the same enum
    // reachable under an alias must not document an event twice.
    let mut documented: BTreeMap<&str, &str> = BTreeMap::new();
    for decl in core.event_enums.values() {
        for (index, variant) in decl.variants.iter().enumerate() {
            if let Some(doc) = decl.docs_of(index).doc.as_deref() {
                if used.contains(variant.as_str()) {
                    documented.entry(variant).or_insert(doc);
                }
            }
        }
    }
    to_documented_names(documented)
}

/// Same for effects, matched by the label transitions carry:
/// `Enum::Variant` for operations, a bare variant name for the root effect
/// enum (crux's `render()` builtin arrives as `Render`).
fn documented_effects(core: &CoreInfo, machines: &[Machine]) -> Vec<DocumentedName> {
    let used: BTreeSet<&str> = machines
        .iter()
        .flat_map(|machine| &machine.transitions)
        .flat_map(|transition| &transition.effects)
        .map(|effect| effect.0.as_str())
        .collect();

    let mut documented: BTreeMap<&str, &str> = BTreeMap::new();
    for label in used {
        let (enum_hint, variant) = match label.split_once("::") {
            Some((enum_name, variant)) => (Some(enum_name), variant),
            None => (None, label),
        };
        let doc = core
            .effect_enums
            .iter()
            .filter(|(name, _)| enum_hint.is_none_or(|hint| hint == name.as_str()))
            .find_map(|(_, decl)| {
                let index = decl.variants.iter().position(|v| v == variant)?;
                decl.docs_of(index).doc.as_deref()
            });
        if let Some(doc) = doc {
            documented.insert(label, doc);
        }
    }
    to_documented_names(documented)
}

fn to_documented_names(entries: BTreeMap<&str, &str>) -> Vec<DocumentedName> {
    entries
        .into_iter()
        .map(|(name, doc)| DocumentedName {
            name: name.to_string(),
            doc: doc.to_string(),
        })
        .collect()
}

/// The model's view of one leaf state. The parser-to-model conversion belongs
/// here, the single place state values are constructed.
fn state_decl(name: &str, docs: &DocBlock) -> StateDecl {
    StateDecl {
        name: name.to_string(),
        doc: docs.doc.clone(),
        markers: docs.markers.clone(),
        tags: docs.tags.clone(),
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
