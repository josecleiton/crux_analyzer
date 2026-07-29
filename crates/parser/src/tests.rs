//! Unit tests: one per extraction pattern, over inline source fixtures.

use crate::loader::sources_from_str;
use crate::parse_sources;
use crux_analyzer_model::Transition;

fn transitions_of(code: &str) -> (Vec<(String, String, String)>, Vec<String>) {
    let sources = sources_from_str(&[("lib.rs", code)]);
    let outcome = parse_sources(&sources, "test").expect("must parse");
    let transitions = outcome.project.cores[0]
        .machines
        .first()
        .map(|machine| machine.transitions.clone())
        .unwrap_or_default();
    (
        transitions
            .iter()
            .map(|t: &Transition| (t.from.0.clone(), t.event.0.clone(), t.to.0.clone()))
            .collect(),
        outcome.warnings.iter().map(|w| w.message.clone()).collect(),
    )
}

fn triple(from: &str, event: &str, to: &str) -> (String, String, String) {
    (from.to_string(), event.to_string(), to.to_string())
}

const PREAMBLE: &str = r#"
    pub enum State { Idle, Running, Done }
    pub struct Model { state: State }
    pub struct App1;
"#;

#[test]
fn guard_plus_assignment() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Start, Finish }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Start if matches!(model.state, State::Idle) => {{
                        model.state = State::Running;
                    }}
                    Event::Finish if matches!(model.state, State::Running) => {{
                        model.state = State::Done;
                    }}
                    _ => {{}}
                }}
            }}
        }}
    "#
    );
    let (transitions, warnings) = transitions_of(&code);
    assert_eq!(
        transitions,
        vec![
            triple("Idle", "Start", "Running"),
            triple("Running", "Finish", "Done"),
        ]
    );
    assert!(warnings.is_empty(), "unexpected warnings: {warnings:?}");
}

#[test]
fn multi_pattern_guard_fans_out() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Stop }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Stop
                        if matches!(model.state, State::Idle | State::Running) => {{
                        model.state = State::Done;
                    }}
                    _ => {{}}
                }}
            }}
        }}
    "#
    );
    let (transitions, _) = transitions_of(&code);
    assert_eq!(
        transitions,
        vec![triple("Idle", "Stop", "Done"), triple("Running", "Stop", "Done")]
    );
}

#[test]
fn multi_event_arm_fans_out() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Cancel, Fail }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    event @ (Event::Cancel | Event::Fail)
                        if matches!(model.state, State::Running) => {{
                        model.state = State::Idle;
                    }}
                    _ => {{}}
                }}
            }}
        }}
    "#
    );
    let (transitions, _) = transitions_of(&code);
    assert_eq!(
        transitions,
        vec![triple("Running", "Cancel", "Idle"), triple("Running", "Fail", "Idle")]
    );
}

#[test]
fn helper_delegation_across_files() {
    let app = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Start }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Start if matches!(model.state, State::Idle) => {{
                        Self::begin(model)
                    }}
                    _ => {{}}
                }}
            }}
        }}
    "#
    );
    let helper = r#"
        impl App1 {
            fn begin(model: &mut Model) {
                model.state = State::Running;
            }
        }
    "#;
    let sources = sources_from_str(&[("app.rs", &app), ("helper.rs", helper)]);
    let outcome = parse_sources(&sources, "test").unwrap();
    let machine = &outcome.project.cores[0].machines[0];
    assert_eq!(machine.transitions.len(), 1);
    assert_eq!(machine.transitions[0].from.0, "Idle");
    assert_eq!(machine.transitions[0].event.0, "Start");
    assert_eq!(machine.transitions[0].to.0, "Running");
}

#[test]
fn nested_event_enum_unwraps_to_leaf() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Inner {{ Go }}
        pub enum Event {{ Wrapped(Inner) }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Wrapped(inner) => Self::update_inner(inner, model),
                }}
            }}
        }}
        impl App1 {{
            fn update_inner(event: Inner, model: &mut Model) {{
                match event {{
                    Inner::Go if matches!(model.state, State::Idle) => {{
                        model.state = State::Running;
                    }}
                    _ => {{}}
                }}
            }}
        }}
    "#
    );
    let (transitions, _) = transitions_of(&code);
    assert_eq!(transitions, vec![triple("Idle", "Go", "Running")]);
}

#[test]
fn match_on_state_with_wildcard_complement() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Reset }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Reset => {{
                        match model.state {{
                            State::Done => {{}}
                            _ => {{
                                model.state = State::Idle;
                            }}
                        }}
                    }}
                }}
            }}
        }}
    "#
    );
    let (transitions, _) = transitions_of(&code);
    // `_` is the complement of the arms above it: Idle and Running.
    assert_eq!(
        transitions,
        vec![triple("Idle", "Reset", "Idle"), triple("Running", "Reset", "Idle")]
    );
}

#[test]
fn payload_variants_normalize_to_name() {
    let code = r#"
        pub enum State { Idle, Busy { automatic: bool } }
        pub struct Model { state: State }
        pub struct App1;
        pub enum Event { Work, Done }
        impl App for App1 {
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {
                match event {
                    Event::Work if matches!(model.state, State::Idle) => {
                        model.state = State::Busy { automatic: false };
                    }
                    Event::Done if matches!(model.state, State::Busy { .. }) => {
                        model.state = State::Idle;
                    }
                    _ => {}
                }
            }
        }
    "#;
    let (transitions, _) = transitions_of(code);
    assert_eq!(
        transitions,
        vec![triple("Idle", "Work", "Busy"), triple("Busy", "Done", "Idle")]
    );
}

#[test]
fn unknown_source_state_warns_instead_of_emitting() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Kill }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Kill if model.state.is_active() => {{
                        model.state = State::Done;
                    }}
                    _ => {{}}
                }}
            }}
        }}
        impl Model {{
            fn probe(&mut self) {{
                if matches!(self.state, State::Idle) {{}}
            }}
        }}
    "#
    );
    let (transitions, warnings) = transitions_of(&code);
    assert!(transitions.is_empty());
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("could not be resolved statically"), "{warnings:?}");
}

#[test]
fn predicate_method_guard_resolves_source_states() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Kill, Revive }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Kill if model.state.is_active() => {{
                        model.state = State::Done;
                    }}
                    Event::Revive if !model.state.is_active() => {{
                        model.state = State::Idle;
                    }}
                    _ => {{}}
                }}
            }}
        }}
        impl State {{
            pub const fn is_active(&self) -> bool {{
                matches!(self, Self::Idle | Self::Running)
            }}
        }}
    "#
    );
    let (transitions, warnings) = transitions_of(&code);
    assert_eq!(
        transitions,
        vec![
            triple("Idle", "Kill", "Done"),
            triple("Running", "Kill", "Done"),
            // negated predicate resolves to the complement
            triple("Done", "Revive", "Idle"),
        ]
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn negated_predicate_with_negated_body_resolves() {
    // `!state.has_anything()` where the body is itself `!matches!(...)`.
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Reset }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Reset if !model.state.has_anything() => {{
                        model.state = State::Running;
                    }}
                    _ => {{}}
                }}
            }}
        }}
        impl State {{
            pub fn has_anything(&self) -> bool {{
                !matches!(self, Self::Idle)
            }}
        }}
    "#
    );
    let (transitions, _) = transitions_of(&code);
    assert_eq!(transitions, vec![triple("Idle", "Reset", "Running")]);
}

#[test]
fn default_reset_lands_on_default_variant() {
    let code = r#"
        pub enum State { #[default] Idle, Running, Done }
        pub struct Session { state: State, count: u32 }
        pub struct Model { session: Session }
        pub struct App1;
        pub enum Event { Discard }
        impl App for App1 {
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {
                match event {
                    Event::Discard if matches!(model.session.state, State::Done) => {
                        model.session = Session::default();
                    }
                    _ => {}
                }
            }
        }
    "#;
    let (transitions, warnings) = transitions_of(code);
    assert_eq!(transitions, vec![triple("Done", "Discard", "Idle")]);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn unguarded_assignment_fires_from_any_state() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Panic }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Panic => {{
                        model.state = State::Idle;
                    }}
                    _ => {{}}
                }}
            }}
        }}
    "#
    );
    let (transitions, warnings) = transitions_of(&code);
    assert_eq!(transitions, vec![triple("*", "Panic", "Idle")]);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn mirror_enum_is_not_a_state_machine() {
    // `Status` mirrors `State` but is never assigned to a model field, so it
    // must not be detected; only `State` transitions are emitted.
    let code = format!(
        r#"{PREAMBLE}
        pub enum Status {{ Idle, Running, Done }}
        pub enum Event {{ Start }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Start if matches!(model.state, State::Idle) => {{
                        model.state = State::Running;
                    }}
                    _ => {{}}
                }}
            }}
            fn view(&self, model: &Model) -> Status {{
                match model.state {{
                    State::Idle => Status::Idle,
                    State::Running => Status::Running,
                    State::Done => Status::Done,
                }}
            }}
        }}
    "#
    );
    let sources = sources_from_str(&[("lib.rs", &code)]);
    let outcome = parse_sources(&sources, "test").unwrap();
    let core = &outcome.project.cores[0];
    assert_eq!(core.machines.len(), 1, "only State must be a machine");
    let machine = &core.machines[0];
    assert_eq!(machine.name, "State");
    assert_eq!(
        machine.states.iter().map(|s| s.0.clone()).collect::<Vec<_>>(),
        vec!["Idle", "Running", "Done"]
    );
    assert_eq!(machine.transitions.len(), 1);
}

#[test]
fn if_matches_narrows_source_state() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Tick }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Tick => {{
                        if matches!(model.state, State::Running) {{
                            model.state = State::Done;
                        }}
                    }}
                }}
            }}
        }}
    "#
    );
    let (transitions, _) = transitions_of(&code);
    assert_eq!(transitions, vec![triple("Running", "Tick", "Done")]);
}

#[test]
fn cfg_test_modules_are_ignored() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Start }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Start if matches!(model.state, State::Idle) => {{
                        model.state = State::Running;
                    }}
                    _ => {{}}
                }}
            }}
        }}
        #[cfg(test)]
        mod tests {{
            fn helper(model: &mut super::Model) {{
                model.state = super::State::Done;
            }}
        }}
    "#
    );
    let (transitions, warnings) = transitions_of(&code);
    assert_eq!(transitions, vec![triple("Idle", "Start", "Running")]);
    assert!(warnings.is_empty(), "test code must not produce warnings: {warnings:?}");
}

#[test]
fn multiple_state_machines_become_regions() {
    let code = r#"
        pub enum State { Idle, Running }
        pub enum NetState { Offline, Online }
        pub struct Model { state: State, net: NetState }
        pub struct App1;
        pub enum Event { Start, Connected }
        impl App for App1 {
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {
                match event {
                    Event::Start if matches!(model.state, State::Idle) => {
                        model.state = State::Running;
                    }
                    Event::Connected if matches!(model.net, NetState::Offline) => {
                        model.net = NetState::Online;
                    }
                    _ => {}
                }
            }
        }
    "#;
    let sources = sources_from_str(&[("lib.rs", code)]);
    let outcome = parse_sources(&sources, "test").unwrap();
    let core = &outcome.project.cores[0];

    assert_eq!(core.machines.len(), 2);
    let names: Vec<&str> = core.machines.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"State") && names.contains(&"NetState"), "{names:?}");
    for machine in &core.machines {
        assert_eq!(machine.transitions.len(), 1, "one transition per region");
    }
}

#[test]
fn no_core_is_an_error() {
    let sources = sources_from_str(&[("lib.rs", "pub struct NotACore;")]);
    assert!(matches!(
        parse_sources(&sources, "test"),
        Err(crate::ParseError::NoCoreFound)
    ));
}
