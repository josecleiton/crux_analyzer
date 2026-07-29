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
        // English is the source locale, so existing prose assertions hold.
        outcome
            .warnings
            .iter()
            .map(|w| w.kind.message(crux_analyzer_i18n::Locale::En))
            .collect(),
    )
}

fn triple(from: &str, event: &str, to: &str) -> (String, String, String) {
    (from.to_string(), event.to_string(), to.to_string())
}

/// The first machine of the first core, for assertions about documentation
/// rather than about transitions.
fn machine_of(code: &str) -> crux_analyzer_model::Machine {
    let sources = sources_from_str(&[("lib.rs", code)]);
    let outcome = parse_sources(&sources, "test").expect("must parse");
    outcome.project.cores[0].machines[0].clone()
}

/// The declaration of one state by name.
fn state_of(machine: &crux_analyzer_model::Machine, name: &str) -> crux_analyzer_model::StateDecl {
    machine
        .states
        .iter()
        .find(|state| state.name == name)
        .unwrap_or_else(|| panic!("no state {name}"))
        .clone()
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
        machine.states.iter().map(|s| s.name.clone()).collect::<Vec<_>>(),
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
fn equality_guard_resolves_source_states() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Start, Abort }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Start if model.state == State::Idle => {{
                        model.state = State::Running;
                    }}
                    Event::Abort if model.state != State::Done => {{
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
        vec![
            triple("Idle", "Start", "Running"),
            triple("Idle", "Abort", "Done"),
            triple("Running", "Abort", "Done"),
        ]
    );
}

#[test]
fn let_else_find_closure_narrows_the_rest_of_the_block() {
    let code = format!(
        r#"{PREAMBLE}
        pub struct Item {{ id: u32, state: State }}
        pub enum Event {{ Pick }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Pick => {{
                        let Some(item) = model.items.iter_mut().find(|item| {{
                            item.id == 1 && item.state == State::Idle
                        }}) else {{
                            return;
                        }};
                        item.state = State::Running;
                    }}
                }}
            }}
        }}
    "#
    );
    let (transitions, warnings) = transitions_of(&code);
    assert_eq!(transitions, vec![triple("Idle", "Pick", "Running")]);
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn event_payload_target_becomes_wildcard() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Sync(State), Reset }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Sync(status) => {{
                        model.state = status;
                    }}
                    Event::Reset => {{
                        model.state = State::Idle;
                    }}
                }}
            }}
        }}
    "#
    );
    let (transitions, warnings) = transitions_of(&code);
    // the payload-driven write lands anywhere the shell decides: to = "*"
    assert_eq!(
        transitions,
        vec![triple("*", "Sync", "*"), triple("*", "Reset", "Idle")]
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn value_flow_resolves_predicate_constrained_targets() {
    let code = format!(
        r#"{PREAMBLE}
        pub struct Item {{ state: State }}
        pub enum Event {{ CarryOver, Reset }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::CarryOver => {{
                        if model.state == State::Idle && is_final(&model.known.state) {{
                            model.state = model.known.state.clone();
                        }}
                    }}
                    Event::Reset => {{
                        model.state = State::Idle;
                    }}
                }}
            }}
        }}
        const fn is_final(state: &State) -> bool {{
            matches!(state, State::Running | State::Done)
        }}
    "#
    );
    let (transitions, warnings) = transitions_of(&code);
    // from: the == on model.state; to: the predicate on model.known.state
    assert_eq!(
        transitions,
        vec![
            triple("Idle", "CarryOver", "Running"),
            triple("Idle", "CarryOver", "Done"),
            triple("*", "Reset", "Idle"),
        ]
    );
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn dynamic_target_without_evidence_still_warns() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Event {{ Restore, Reset }}
        impl App for App1 {{
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Restore => {{
                        model.state = model.backup.state.clone();
                    }}
                    Event::Reset => {{
                        model.state = State::Idle;
                    }}
                }}
            }}
        }}
    "#
    );
    let (transitions, warnings) = transitions_of(&code);
    assert_eq!(transitions, vec![triple("*", "Reset", "Idle")]);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("target state is dynamic"), "{warnings:?}");
}

#[test]
fn composite_states_expand_to_slash_paths() {
    let code = r#"
        pub enum Phase { Loading, Ready }
        pub enum State { Idle, Active(Phase) }
        pub struct Model { state: State }
        pub struct App1;
        pub enum Event { Start, Loaded, Stop }
        impl App for App1 {
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {
                match event {
                    Event::Start if matches!(model.state, State::Idle) => {
                        model.state = State::Active(Phase::Loading);
                    }
                    Event::Loaded if matches!(model.state, State::Active(Phase::Loading)) => {
                        model.state = State::Active(Phase::Ready);
                    }
                    Event::Stop if matches!(model.state, State::Active(_)) => {
                        model.state = State::Idle;
                    }
                    _ => {}
                }
            }
        }
    "#;
    let sources = sources_from_str(&[("lib.rs", code)]);
    let outcome = parse_sources(&sources, "test").unwrap();
    let machine = &outcome.project.cores[0].machines[0];

    assert_eq!(
        machine.states.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["Idle", "Active/Loading", "Active/Ready"]
    );
    let triples: Vec<(String, String, String)> = machine
        .transitions
        .iter()
        .map(|t| (t.from.0.clone(), t.event.0.clone(), t.to.0.clone()))
        .collect();
    assert_eq!(
        triples,
        vec![
            triple("Idle", "Start", "Active/Loading"),
            triple("Active/Loading", "Loaded", "Active/Ready"),
            // `Active(_)` fans out over every child
            triple("Active/Loading", "Stop", "Idle"),
            triple("Active/Ready", "Stop", "Idle"),
        ]
    );
    assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
}

#[test]
fn effects_attach_to_their_event_arm() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Operation {{ Start, Stop }}
        pub enum Effect {{ Render(RenderOperation), Op(Operation) }}
        pub enum Event {{ Go, Halt }}
        impl App for App1 {{
            type Event = Event;
            type Effect = Effect;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Go if matches!(model.state, State::Idle) => {{
                        model.state = State::Running;
                        render();
                        Self::op(Operation::Start)
                    }}
                    Event::Halt if matches!(model.state, State::Running) => {{
                        model.state = State::Done;
                        Self::op(Operation::Stop)
                    }}
                    _ => {{}}
                }}
            }}
        }}
    "#
    );
    let sources = sources_from_str(&[("lib.rs", &code)]);
    let outcome = parse_sources(&sources, "test").unwrap();
    let machine = &outcome.project.cores[0].machines[0];

    let go = machine.transitions.iter().find(|t| t.event.0 == "Go").unwrap();
    assert_eq!(
        go.effects.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(),
        ["Render", "Operation::Start"]
    );
    let halt = machine.transitions.iter().find(|t| t.event.0 == "Halt").unwrap();
    assert_eq!(
        halt.effects.iter().map(|e| e.0.as_str()).collect::<Vec<_>>(),
        ["Operation::Stop"]
    );
}

#[test]
fn documented_events_and_effects_reach_the_core_catalogs() {
    let code = format!(
        r#"{PREAMBLE}
        pub enum Operation {{
            /// Starts the capture pipeline.
            Start,
            Stop,
        }}
        pub enum Effect {{ Render(RenderOperation), Op(Operation) }}
        pub enum Event {{
            /// The user pressed the record button.
            Go,
            /// Never fires a transition, so it must stay out of the catalog.
            Unused,
            Halt,
        }}
        impl App for App1 {{
            type Event = Event;
            type Effect = Effect;
            fn update(&self, event: Event, model: &mut Model) {{
                match event {{
                    Event::Go if matches!(model.state, State::Idle) => {{
                        model.state = State::Running;
                        Self::op(Operation::Start)
                    }}
                    Event::Halt if matches!(model.state, State::Running) => {{
                        model.state = State::Done;
                        Self::op(Operation::Stop)
                    }}
                    _ => {{}}
                }}
            }}
        }}
    "#
    );
    let sources = sources_from_str(&[("lib.rs", &code)]);
    let outcome = parse_sources(&sources, "test").unwrap();
    let core = &outcome.project.cores[0];

    // Only documented AND used names enter the catalogs: `Unused` is
    // documented but fires nothing, `Halt` fires but says nothing.
    let events: Vec<(&str, &str)> =
        core.events.iter().map(|e| (e.name.as_str(), e.doc.as_str())).collect();
    assert_eq!(events, [("Go", "The user pressed the record button.")]);

    let effects: Vec<(&str, &str)> =
        core.effects.iter().map(|e| (e.name.as_str(), e.doc.as_str())).collect();
    assert_eq!(effects, [("Operation::Start", "Starts the capture pipeline.")]);
    assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
}

#[test]
fn same_enum_in_two_fields_is_two_machines() {
    let code = r#"
        pub enum State { Idle, Running }
        pub struct Model { left: State, right: State }
        pub struct App1;
        pub enum Event { StartLeft, StartRight }
        impl App for App1 {
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {
                match event {
                    Event::StartLeft if matches!(model.left, State::Idle) => {
                        model.left = State::Running;
                    }
                    Event::StartRight if matches!(model.right, State::Idle) => {
                        model.right = State::Running;
                    }
                    _ => {}
                }
            }
        }
    "#;
    let sources = sources_from_str(&[("lib.rs", code)]);
    let outcome = parse_sources(&sources, "test").unwrap();
    let core = &outcome.project.cores[0];

    assert_eq!(core.machines.len(), 2, "each field is its own region");
    let names: Vec<&str> = core.machines.iter().map(|m| m.name.as_str()).collect();
    assert!(
        names.contains(&"State (left)") && names.contains(&"State (right)"),
        "{names:?}"
    );
    for machine in &core.machines {
        assert_eq!(machine.transitions.len(), 1, "one transition per region: {names:?}");
        let expected_event = if machine.name.contains("left") { "StartLeft" } else { "StartRight" };
        assert_eq!(machine.transitions[0].event.0, expected_event);
    }
}

#[test]
fn payload_data_enum_is_not_a_composite_state() {
    // `Failed(ErrorCode)` carries data — no nested variant pattern exists,
    // so Failed stays a plain leaf and the runtime payload does not warn.
    let code = r#"
        pub enum ErrorCode { NotFound, Timeout }
        pub enum State { Idle, Failed(ErrorCode) }
        pub struct Model { state: State }
        pub struct App1;
        pub enum Event { Boom(ErrorCode) }
        impl App for App1 {
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {
                match event {
                    Event::Boom(code) if matches!(model.state, State::Idle) => {
                        model.state = State::Failed(code);
                    }
                    _ => {}
                }
            }
        }
    "#;
    let sources = sources_from_str(&[("lib.rs", code)]);
    let outcome = parse_sources(&sources, "test").unwrap();
    let machine = &outcome.project.cores[0].machines[0];

    assert_eq!(
        machine.states.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["Idle", "Failed"],
        "ErrorCode must not expand into fake sub-states"
    );
    assert_eq!(machine.transitions.len(), 1);
    assert_eq!(machine.transitions[0].to.0, "Failed");
    assert!(outcome.warnings.is_empty(), "{:?}", outcome.warnings);
}

#[test]
fn boxed_nested_event_enum_still_delegates() {
    let code = r#"
        pub enum State { Idle, Running }
        pub struct Model { state: State }
        pub struct App1;
        pub enum Inner { Go }
        pub enum Event { Wrapped(Box<Inner>) }
        impl App for App1 {
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {
                match event {
                    Event::Wrapped(inner) => Self::update_inner(*inner, model),
                }
            }
        }
        impl App1 {
            fn update_inner(event: Inner, model: &mut Model) {
                match event {
                    Inner::Go if matches!(model.state, State::Idle) => {
                        model.state = State::Running;
                    }
                    _ => {}
                }
            }
        }
    "#;
    let (transitions, _) = {
        let sources = sources_from_str(&[("lib.rs", code)]);
        let outcome = parse_sources(&sources, "test").unwrap();
        let t = outcome.project.cores[0].machines[0]
            .transitions
            .iter()
            .map(|t| (t.from.0.clone(), t.event.0.clone(), t.to.0.clone()))
            .collect::<Vec<_>>();
        (t, outcome.warnings)
    };
    // the Box around Inner must not break wrapper detection: the leaf label
    // is Go, not Wrapped
    assert_eq!(transitions, vec![triple("Idle", "Go", "Running")]);
}

#[test]
fn no_core_is_an_error() {
    let sources = sources_from_str(&[("lib.rs", "pub struct NotACore;")]);
    assert!(matches!(
        parse_sources(&sources, "test"),
        Err(crate::ParseError::NoCoreFound)
    ));
}

#[test]
fn variant_docs_become_state_documentation() {
    let machine = machine_of(
        r#"
        pub enum State {
            /// Nothing is being recorded yet.
            Idle,
            /// Capturing audio from the microphone.
            Running,
            Done,
        }
        pub struct Model { state: State }
        pub struct App1;
        pub enum Event { Go }
        impl App for App1 {
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {
                match event {
                    Event::Go if matches!(model.state, State::Idle) => {
                        model.state = State::Running;
                    }
                    _ => {}
                }
            }
        }
        "#,
    );
    assert_eq!(
        state_of(&machine, "Idle").doc.as_deref(),
        Some("Nothing is being recorded yet.")
    );
    assert_eq!(
        state_of(&machine, "Running").doc.as_deref(),
        Some("Capturing audio from the microphone.")
    );
    // An undocumented state stays exactly what it was before documentation.
    assert!(state_of(&machine, "Done").is_bare());
}

#[test]
fn enum_doc_becomes_the_machine_description() {
    let machine = machine_of(&format!(
        r#"
        /// Where a recording session lives.
        ///
        /// @deprecated
        /// @tag legacy
        pub enum State {{ Idle, Running, Done }}
        pub struct Model {{ state: State }}
        pub struct App1;
        {}
        "#,
        UPDATE_GO
    ));
    assert_eq!(
        machine.doc.as_deref(),
        Some("Where a recording session lives.")
    );
    assert_eq!(machine.markers, [crux_analyzer_model::Marker::Deprecated]);
    assert_eq!(machine.tags, ["legacy"]);
}

#[test]
fn declared_markers_and_tags_reach_the_state() {
    let machine = machine_of(&format!(
        r#"
        pub enum State {{
            Idle,
            /// The upload failed. The session is kept so the user can retry.
            ///
            /// @failure
            /// @tag retryable
            Running,
            /// @deprecated
            Done,
        }}
        pub struct Model {{ state: State }}
        pub struct App1;
        {}
        "#,
        UPDATE_GO
    ));
    let running = state_of(&machine, "Running");
    assert_eq!(
        running.doc.as_deref(),
        Some("The upload failed. The session is kept so the user can retry.")
    );
    assert_eq!(running.markers, [crux_analyzer_model::Marker::Failure]);
    assert_eq!(running.tags, ["retryable"]);

    // A marker with no prose is still a documented state.
    let done = state_of(&machine, "Done");
    assert!(done.doc.is_none());
    assert_eq!(done.markers, [crux_analyzer_model::Marker::Deprecated]);
    assert!(done.is_documented());
}

#[test]
fn composite_children_inherit_the_parent_documentation() {
    let machine = machine_of(
        r#"
        /// The machine.
        pub enum State {
            /// Nothing yet.
            Idle,
            /// A session is live.
            ///
            /// @deprecated
            /// @tag region
            Active(Phase),
        }
        pub enum Phase {
            /// Fetching the manifest.
            ///
            /// @failure
            Loading,
            Ready,
        }
        pub struct Model { state: State }
        pub struct App1;
        pub enum Event { Go }
        impl App for App1 {
            type Event = Event;
            fn update(&self, event: Event, model: &mut Model) {
                match event {
                    Event::Go if matches!(model.state, State::Active(Phase::Loading)) => {
                        model.state = State::Idle;
                    }
                    _ => {}
                }
            }
        }
        "#,
    );
    let loading = state_of(&machine, "Active/Loading");
    // Parent prose first, then the leaf's — the parent has no node of its own.
    assert_eq!(
        loading.doc.as_deref(),
        Some("A session is live.\n\nFetching the manifest.")
    );
    assert_eq!(
        loading.markers,
        [
            crux_analyzer_model::Marker::Deprecated,
            crux_analyzer_model::Marker::Failure
        ]
    );
    assert_eq!(loading.tags, ["region"]);

    // An undocumented child still inherits the superstate's statement.
    let ready = state_of(&machine, "Active/Ready");
    assert_eq!(ready.doc.as_deref(), Some("A session is live."));
    assert_eq!(ready.markers, [crux_analyzer_model::Marker::Deprecated]);
}

#[test]
fn ordinary_documentation_never_warns() {
    let (_, warnings) = transitions_of(&format!(
        r#"
        /// Prose that mentions `@Generable` mid-sentence, and an address like
        /// support@example.com, and a fenced sample:
        ///
        /// ```
        /// @failure
        /// ```
        pub enum State {{ Idle, Running, Done }}
        pub struct Model {{ state: State }}
        pub struct App1;
        {}
        "#,
        UPDATE_GO
    ));
    assert!(warnings.is_empty(), "{warnings:?}");
}

#[test]
fn an_unrecognized_annotation_warns_once_per_line() {
    let (_, warnings) = transitions_of(&format!(
        r#"
        pub enum State {{
            /// @failur
            Idle,
            Running,
            Done,
        }}
        pub struct Model {{ state: State }}
        pub struct App1;
        {}
        "#,
        UPDATE_GO
    ));
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("@failur"), "{warnings:?}");
}

/// A doc comment on an enum that is not a state machine must stay silent —
/// warnings are only worth paying for where the annotation would have meant
/// something.
#[test]
fn annotations_on_a_non_machine_enum_are_ignored() {
    let (_, warnings) = transitions_of(&format!(
        r#"
        /// @nonsense
        pub enum ViewModel {{ Empty, Full }}
        pub enum State {{ Idle, Running, Done }}
        pub struct Model {{ state: State }}
        pub struct App1;
        {}
        "#,
        UPDATE_GO
    ));
    assert!(warnings.is_empty(), "{warnings:?}");
}

/// An aliased state enum is indexed twice; one typo must still warn once.
#[test]
fn an_aliased_enum_does_not_duplicate_its_warning() {
    let sources = sources_from_str(&[
        (
            "state.rs",
            r#"
            pub enum State {
                /// @failur
                Idle,
                Running,
            }
            "#,
        ),
        (
            "lib.rs",
            r#"
            use crate::state::State as RecorderState;
            pub struct Model { state: RecorderState }
            pub struct App1;
            pub enum Event { Go }
            impl App for App1 {
                type Event = Event;
                fn update(&self, event: Event, model: &mut Model) {
                    match event {
                        Event::Go if matches!(model.state, RecorderState::Idle) => {
                            model.state = RecorderState::Running;
                        }
                        _ => {}
                    }
                }
            }
            "#,
        ),
    ]);
    let outcome = parse_sources(&sources, "test").expect("must parse");
    let annotation_warnings: Vec<_> = outcome
        .warnings
        .iter()
        .filter(|w| w.kind.code() == "unknown-annotation")
        .collect();
    assert_eq!(annotation_warnings.len(), 1, "{:?}", outcome.warnings);
}

/// A state machine with no documentation at all must produce the same states
/// it produced before documentation existed — the byte-identity guard.
#[test]
fn undocumented_states_carry_no_metadata() {
    let machine = machine_of(&format!("{PREAMBLE}{UPDATE_GO}"));
    assert!(machine.doc.is_none());
    assert!(machine.markers.is_empty());
    assert!(machine.tags.is_empty());
    assert!(machine.states.iter().all(|s| s.is_bare()), "{:?}", machine.states);
}

/// The `update` body shared by the documentation tests above: one transition,
/// enough to make `State` a machine.
const UPDATE_GO: &str = r#"
    pub enum Event { Go }
    impl App for App1 {
        type Event = Event;
        fn update(&self, event: Event, model: &mut Model) {
            match event {
                Event::Go if matches!(model.state, State::Idle) => {
                    model.state = State::Running;
                }
                _ => {}
            }
        }
    }
"#;
