//! Documentation generators for the crux_analyzer semantic model, and the
//! measure of how much of it is documented ([`coverage`]).
//!
//! Every generator consumes only [`crux_analyzer_model`] types — never the
//! parser or its AST — so they work for any client that has a model JSON.
//!
//! Generators take a [`Locale`](crux_analyzer_i18n::Locale) and render their
//! prose through [`Labels`]. Everything else they emit is Markdown/Mermaid
//! syntax or model data, which is locale-independent by contract.

use std::collections::HashMap;

use crux_analyzer_model::{Machine, Marker, State, StateDecl, Transition};

mod coverage;
mod labels;
mod markdown;
mod mermaid;
mod roles;

pub use coverage::{coverage, Coverage, MachineCoverage, ProjectCoverage};
pub use labels::Labels;
pub use markdown::markdown;
pub use mermaid::{mermaid_diagrams, Diagram};
pub use roles::MachineRoles;

/// Words Mermaid's state-diagram grammar claims for itself. A state named after
/// one of these cannot be a bare node id — `enum State { end }` is legal Rust,
/// and `end` alone would break the whole diagram.
const MERMAID_KEYWORDS: &[&str] = &[
    "state",
    "note",
    "end",
    "direction",
    "class",
    "classDef",
    "click",
    "style",
    "as",
    "stateDiagram",
    "left",
    "right",
    "of",
];

/// Whether `name` can be used verbatim as a Mermaid node id.
///
/// Everything else — composite paths (`Active/Loading`), raw identifiers
/// (`r#type`), non-ASCII identifiers, keywords — gets a generated id plus a
/// quoted label, so the diagram keeps rendering and still shows the real name.
fn is_safe_bare_id(name: &str) -> bool {
    !name.is_empty()
        && name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !MERMAID_KEYWORDS.contains(&name)
}

/// Node ids for one machine: stable, collision-free, and identical to the state
/// name whenever that name is already a safe id.
///
/// Built per machine because collision resolution needs to see every name at
/// once — `Active/Loading` and a sibling variant literally named
/// `Active_Loading` would otherwise both become `Active_Loading` and silently
/// merge into one node.
struct Ids {
    by_state: HashMap<String, String>,
}

impl Ids {
    fn build(machine: &Machine) -> Self {
        let mut by_state: HashMap<String, String> = HashMap::new();
        let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();
        // `any_state` is referenced by generated lines, so it is reserved even
        // when no state is called that.
        taken.insert("any_state".to_string());

        // States first, then composite parents (which are node ids too, though
        // no state declares them), then transition endpoints: a `to` resolved
        // at runtime may name something the enum does not declare.
        let names = machine
            .states
            .iter()
            .map(|s| s.name.as_str())
            .chain(
                machine
                    .states
                    .iter()
                    .filter_map(|s| s.name.split_once('/').map(|(parent, _)| parent)),
            )
            .chain(
                machine
                    .transitions
                    .iter()
                    .flat_map(|t| [t.from.0.as_str(), t.to.0.as_str()]),
            );

        for name in names {
            if name == State::ANY || by_state.contains_key(name) {
                continue;
            }
            // Sanitize first, then prefix only if the result is still unusable:
            // `Active/Loading` keeps its readable `Active_Loading` id, while a
            // keyword (`end`) or a leading digit has to be prefixed.
            let sanitized = sanitize_id(name);
            let mut candidate = if is_safe_bare_id(&sanitized) {
                sanitized
            } else {
                format!("s_{sanitized}")
            };
            let base = candidate.clone();
            let mut suffix = 2;
            while !taken.insert(candidate.clone()) {
                candidate = format!("{base}_{suffix}");
                suffix += 1;
            }
            by_state.insert(name.to_string(), candidate);
        }

        Self { by_state }
    }

    fn id(&self, state: &str) -> &str {
        if state == State::ANY {
            return "any_state";
        }
        self.by_state.get(state).map_or("any_state", String::as_str)
    }

    /// Whether this state needs a `state "Name" as id` line for its real name to
    /// appear in the diagram.
    fn needs_label(&self, state: &str) -> bool {
        state != State::ANY && self.id(state) != state
    }
}

/// Escapes model text for a Mermaid label or note.
///
/// Mermaid has no backslash escape; it has *entity codes* (`#quot;`, `#60;`),
/// which it renders back to the character. Four things have to go:
///
/// - `"` would close a quoted label;
/// - `<` and `>` would be markup in a renderer configured with `htmlLabels`;
/// - `%%` starts a comment that swallows the rest of the line;
/// - newlines, via [`one_line`], would inject a diagram statement.
fn mermaid_label(text: &str) -> String {
    one_line(text)
        .replace('"', "#quot;")
        .replace('<', "#60;")
        .replace('>', "#62;")
        .replace("%%", "%\u{200b}%")
}

/// Flattens text to a single line and drops control characters.
///
/// Every Mermaid statement is line-terminated, so an embedded newline in author
/// prose or an identifier would inject a diagram line. See `docs/security.md`.
pub(crate) fn one_line(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_control())
        .collect()
}

/// One `stateDiagram-v2` body for a machine (without code fences).
/// Composite leaves render nested inside their parent's block.
fn machine_diagram(machine: &Machine, labels: &Labels) -> String {
    let ids = Ids::build(machine);
    let mut lines = vec!["stateDiagram-v2".to_string()];

    if machine
        .transitions
        .iter()
        .any(|t| t.from.0 == State::ANY || t.to.0 == State::ANY)
    {
        // Only the quoted label is localized: `any_state` is the node id the
        // transition lines refer to, so it must stay stable across locales.
        lines.push(format!(
            "    state \"{}\" as any_state",
            mermaid_label(labels.any_state)
        ));
    }

    // Which states a transition already declares by mentioning them, computed
    // once: asking per state turned this into a states × transitions scan.
    let referenced: std::collections::HashSet<&str> = machine
        .transitions
        .iter()
        .flat_map(|t| [t.from.0.as_str(), t.to.0.as_str()])
        .collect();

    // Composite children grouped by parent in one pass, preserving declaration
    // order — the previous shape rescanned every state for each parent.
    let mut parents: Vec<&str> = Vec::new();
    let mut children: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
    for state in &machine.states {
        if let Some((parent, child)) = state.name.split_once('/') {
            if !children.contains_key(parent) {
                parents.push(parent);
            }
            children
                .entry(parent)
                .or_default()
                .push((child, state.name.as_str()));
        }
    }

    // Composite blocks: `state Parent { state "Child" as Parent_Child }`.
    for parent in &parents {
        // A parent whose name is not a usable id is declared with a quoted
        // label, the same way its children are.
        if ids.needs_label(parent) {
            lines.push(format!(
                "    state \"{}\" as {} {{",
                mermaid_label(parent),
                ids.id(parent)
            ));
        } else {
            lines.push(format!("    state {parent} {{"));
        }
        for (child, full) in &children[*parent] {
            lines.push(format!(
                "        state \"{}\" as {}",
                mermaid_label(child),
                ids.id(full)
            ));
        }
        lines.push("    }".to_string());
    }

    for state in &machine.states {
        if state.name.contains('/') {
            continue; // declared inside its composite block
        }
        if ids.needs_label(&state.name) {
            // The name cannot be a bare id, so the label carries it.
            lines.push(format!(
                "    state \"{}\" as {}",
                mermaid_label(&state.name),
                ids.id(&state.name)
            ));
        } else if !referenced.contains(state.name.as_str()) {
            // Orphan simple states still show up in the diagram.
            lines.push(format!("    {}", ids.id(&state.name)));
        }
    }

    for transition in &machine.transitions {
        lines.push(format!(
            "    {} --> {}: {}",
            ids.id(&transition.from.0),
            ids.id(&transition.to.0),
            mermaid_label(&transition_label(transition)),
        ));
    }

    // The two derived roles, as the pseudo-state the state-diagram grammar has
    // for exactly this. `[*]` needs no id and no label, so it is the one part of
    // a diagram that carries no data from the analyzed application. A composite
    // child that is an entry point gets its arrow here, in the outer region, the
    // same way a transition into it is drawn at this level.
    let roles = MachineRoles::of(machine);
    for state in &machine.states {
        if roles.is_initial(&state.name) {
            lines.push(format!("    [*] --> {}", ids.id(&state.name)));
        }
    }
    for state in &machine.states {
        if roles.is_final(&state.name) {
            lines.push(format!("    {} --> [*]", ids.id(&state.name)));
        }
    }

    // Notes last: by now every id the diagram uses has been declared, whether
    // by a transition line, the orphan guard or a composite block — so this
    // needs no declaration line of its own and the orphan shortcut above
    // stays exactly as it is.
    for state in &machine.states {
        if let Some(doc) = &state.doc {
            lines.push(format!(
                "    note right of {} : {}",
                ids.id(&state.name),
                note_text(doc),
            ));
        }
    }

    lines.join("\n")
}

fn sanitize_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// A doc comment as a one-line Mermaid note.
///
/// The diagram is a hint — the states table below it carries the whole text —
/// so this takes the first paragraph and truncates on a word boundary. It is
/// the only place documentation is shortened.
fn note_text(doc: &str) -> String {
    let first = doc.split("\n\n").next().unwrap_or(doc);
    // A note runs to end of line, so `%%` in author prose would comment out the
    // rest of it — and a newline would inject a diagram statement.
    let flat = mermaid_label(first);
    if flat.chars().count() <= NOTE_MAX_CHARS {
        return flat;
    }
    let mut out: String = flat.chars().take(NOTE_MAX_CHARS).collect();
    if let Some(space) = out.rfind(' ') {
        out.truncate(space);
    }
    out.push('…');
    out
}

const NOTE_MAX_CHARS: usize = 72;

/// The localized name of a marker.
///
/// An exhaustive match on purpose: a new [`Marker`] must not compile until
/// every locale has a word for it.
fn marker_label(marker: Marker, labels: &Labels) -> &'static str {
    match marker {
        Marker::Failure => labels.marker_failure,
        Marker::Deprecated => labels.marker_deprecated,
    }
}

/// Whether a machine has anything to put in a states table.
fn has_documented_states(machine: &Machine) -> bool {
    machine.states.iter().any(StateDecl::is_documented)
}

/// The transition table's effects cell: each request, the event it is answered
/// with when the source names one, and a qualifier when the transition's own
/// path does not imply it.
fn effects_cell(transition: &Transition, labels: &Labels) -> String {
    if transition.effects.is_empty() {
        return labels.no_effects.to_string();
    }
    transition
        .effects
        .iter()
        .map(|effect| {
            let mut cell = format!("`{}`", effect.name);
            if !effect.resolves_with.is_empty() {
                cell.push_str(&format!(
                    " → {}",
                    answers_cell(&effect.resolves_with, labels)
                ));
            }
            if effect.conditional {
                cell.push_str(&format!(" ({})", labels.conditional));
            }
            cell
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Event names as a monospace, comma-separated list.
fn monospace_events(events: &[crux_analyzer_model::Event]) -> String {
    events
        .iter()
        .map(|event| format!("`{}`", event.0))
        .collect::<Vec<_>>()
        .join(", ")
}

/// How many answers a transition row lists before counting the rest. A request
/// built by a shared helper can be answered by every event that helper's
/// callback maps, which is a dozen in a real app — too many for a table cell,
/// and never dropped silently: the count says how many were left out and the
/// capabilities table lists them all.
const ANSWERS_IN_A_CELL: usize = 3;

fn answers_cell(events: &[crux_analyzer_model::Event], labels: &Labels) -> String {
    if events.len() <= ANSWERS_IN_A_CELL {
        return monospace_events(events);
    }
    format!(
        "{}, +{} {}",
        monospace_events(&events[..ANSWERS_IN_A_CELL]),
        events.len() - ANSWERS_IN_A_CELL,
        labels.more
    )
}

/// A transition's Mermaid label: `event / effect, effect` — the statechart
/// convention for "this event, and what firing it asks the shell to do".
///
/// Deliberately terser than the table cell: a conditional request is marked with
/// a `?` and the callback event is left to the table, because a diagram edge has
/// to stay readable at a glance.
fn transition_label(transition: &Transition) -> String {
    if transition.effects.is_empty() {
        return transition.event.0.clone();
    }
    let effects = transition
        .effects
        .iter()
        .map(|effect| {
            if effect.conditional {
                format!("{}?", effect.name)
            } else {
                effect.name.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{} / {}", transition.event.0, effects)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crux_analyzer_i18n::Locale;
    use crux_analyzer_model::{Core, DocumentedName, Effect, Event, Project};

    fn sample() -> Project {
        Project {
            project: "Sample".into(),
            cores: vec![Core {
                name: "Player".into(),
                machines: vec![Machine {
                    name: "PlayerState".into(),
                    states: vec!["Stopped".into(), "Playing".into(), "Orphan".into()],
                    transitions: vec![
                        Transition {
                            from: State("Stopped".into()),
                            event: Event("Play".into()),
                            to: State("Playing".into()),
                            effects: vec![Effect::bare("Render"), Effect::bare("Audio::Start")],
                        },
                        Transition {
                            from: State(State::ANY.into()),
                            event: Event("Reset".into()),
                            to: State("Stopped".into()),
                            effects: vec![],
                        },
                    ],
                    ..Default::default()
                }],
                ..Default::default()
            }],
        }
    }

    /// `sample()` with documentation on it. Kept separate so the undocumented
    /// project stays available as the backwards-compatibility guard.
    fn documented_sample() -> Project {
        let mut project = sample();
        let machine = &mut project.cores[0].machines[0];
        machine.doc = Some("Plays one track at a time.".into());
        machine.states[0] = StateDecl {
            name: "Stopped".into(),
            doc: Some("Nothing is playing.\n\nThe track stays loaded, so\nplay resumes it.".into()),
            markers: vec![],
            tags: vec!["idle-ish".into()],
            ..Default::default()
        };
        machine.states[1] = StateDecl {
            name: "Playing".into(),
            doc: Some("Audio is reaching the speakers.".into()),
            markers: vec![Marker::Failure, Marker::Deprecated],
            tags: vec![],
            ..Default::default()
        };
        project
    }

    /// The diagram states both derived roles. Before this, `[*]` appeared in no
    /// generated document at all: the web painted an entry and a dead end that
    /// the Mermaid and the Markdown never mentioned.
    #[test]
    fn mermaid_marks_the_entry_point_and_the_dead_ends() {
        let body = &mermaid_diagrams(&sample(), Locale::En)[0].mermaid;
        // `Stopped` is where the wildcard transition arrives, so the entry point
        // is the state nothing reaches.
        assert!(body.contains("\n    [*] --> Orphan"), "{body}");
        assert!(!body.contains("[*] --> Stopped"), "{body}");
        assert!(body.contains("\n    Playing --> [*]"), "{body}");
        assert!(body.contains("\n    Orphan --> [*]"), "{body}");
        assert!(!body.contains("Stopped --> [*]"), "{body}");
    }

    /// A `#[default]` variant is the entry point even when the machine is a
    /// cycle, which is the case declaration order cannot answer.
    #[test]
    fn mermaid_takes_the_entry_point_from_the_declared_default() {
        let mut project = sample();
        let machine = &mut project.cores[0].machines[0];
        machine.transitions.push(Transition {
            from: State("Playing".into()),
            event: Event("Stop".into()),
            to: State("Orphan".into()),
            effects: vec![],
        });
        machine.states[1].is_default = true;

        let body = &mermaid_diagrams(&project, Locale::En)[0].mermaid;
        assert!(body.contains("\n    [*] --> Playing"), "{body}");
        assert_eq!(body.matches("[*] -->").count(), 1, "{body}");
    }

    #[test]
    fn mermaid_notes_document_states() {
        let body = &mermaid_diagrams(&documented_sample(), Locale::En)[0].mermaid;
        assert!(
            body.contains("note right of Playing : Audio is reaching the speakers."),
            "{body}"
        );
        // Undocumented states get no note.
        assert!(!body.contains("note right of Orphan"), "{body}");
        // Notes come after the transitions, so every id is declared by then.
        let first_transition = body.find("-->").expect("a transition line");
        let first_note = body.find("note right of").expect("a note line");
        assert!(first_note > first_transition, "{body}");
    }

    #[test]
    fn mermaid_notes_only_the_first_paragraph_on_one_line() {
        let body = &mermaid_diagrams(&documented_sample(), Locale::En)[0].mermaid;
        let note = body
            .lines()
            .find(|line| line.contains("note right of Stopped"))
            .expect("a note for Stopped");
        assert!(note.ends_with("Nothing is playing."), "{note}");
    }

    #[test]
    fn mermaid_truncates_a_long_state_doc() {
        let mut project = sample();
        project.cores[0].machines[0].states[1].doc =
            Some("word ".repeat(80).trim_end().to_string());
        let body = &mermaid_diagrams(&project, Locale::En)[0].mermaid;
        let note = body
            .lines()
            .find(|line| line.contains("note right of Playing"))
            .expect("a note");
        assert!(note.ends_with('…'), "{note}");
        assert!(note.chars().count() < 110, "{note}");
    }

    /// Documenting a state must not make it declare itself: the orphan
    /// shortcut is what keeps the diagram lean.
    #[test]
    fn mermaid_keeps_the_orphan_shortcut_when_states_are_documented() {
        let body = &mermaid_diagrams(&documented_sample(), Locale::En)[0].mermaid;
        assert!(!body.contains("state \"Playing\" as Playing"), "{body}");
        assert!(body.contains("\n    Orphan"), "{body}");
    }

    #[test]
    fn mermaid_notes_a_composite_child_by_its_flattened_id() {
        let mut project = Project {
            project: "S".into(),
            cores: vec![Core {
                name: "C".into(),
                machines: vec![Machine {
                    name: "State".into(),
                    states: vec!["Idle".into(), "Active/Loading".into()],
                    transitions: vec![Transition {
                        from: State("Idle".into()),
                        event: Event("Start".into()),
                        to: State("Active/Loading".into()),
                        effects: vec![],
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            }],
        };
        project.cores[0].machines[0].states[1].doc = Some("Fetching the manifest.".into());
        let body = &mermaid_diagrams(&project, Locale::En)[0].mermaid;
        assert!(
            body.contains("note right of Active_Loading : Fetching the manifest."),
            "{body}"
        );
    }

    /// Markers are words in a table, not colours in a diagram: a `classDef`
    /// fill would hardcode a hex that breaks in a dark-mode reader.
    #[test]
    fn mermaid_renders_no_marker_styling() {
        let body = &mermaid_diagrams(&documented_sample(), Locale::En)[0].mermaid;
        assert!(!body.contains("classDef"), "{body}");
        assert!(!body.contains("failure"), "{body}");
    }

    #[test]
    fn markdown_lists_documented_states() {
        let doc = markdown(&documented_sample(), Locale::En);
        assert!(doc.contains("#### States"), "{doc}");
        assert!(
            doc.contains("| State | Role | Description | Markers | Tags |"),
            "{doc}"
        );
        assert!(
            doc.contains(
                "| Playing | final | Audio is reaching the speakers. | failure, deprecated | — |"
            ),
            "{doc}"
        );
        assert!(
            doc.contains("| Stopped | — | Nothing is playing. | — | `idle-ish` |"),
            "{doc}"
        );
        // An undocumented state still has a row, so the table is the state list.
        assert!(doc.contains("| Orphan | initial, final | — | — | — |"), "{doc}");
    }

    #[test]
    fn markdown_renders_documented_events_and_effects_per_core() {
        let mut project = sample();
        project.cores[0].events = vec![DocumentedName {
            name: "Play".into(),
            doc: "Starts playback.\n\nQueues the track first.".into(),
        }];
        project.cores[0].effects = vec![DocumentedName {
            name: "Audio::Start".into(),
            doc: "Begins capture.".into(),
        }];

        let en = markdown(&project, Locale::En);
        assert!(en.contains("### Events"), "{en}");
        // the whole description survives, unwrapped into the cell
        assert!(en.contains("| `Play` | Starts playback. Queues the track first. |"), "{en}");
        assert!(en.contains("### Effects"), "{en}");
        assert!(en.contains("| `Audio::Start` | Begins capture. |"), "{en}");

        // headings translate; names and author prose never do
        let pt = markdown(&project, Locale::PtBr);
        assert!(pt.contains("### Eventos"), "{pt}");
        assert!(pt.contains("### Efeitos"), "{pt}");
        assert!(pt.contains("| `Play` | Starts playback. Queues the track first. |"), "{pt}");

        // the undocumented project emits no catalog at all
        let bare = markdown(&sample(), Locale::En);
        assert!(!bare.contains("### Events"), "{bare}");
    }

    #[test]
    fn markdown_renders_the_machine_description_above_the_diagram() {
        let doc = markdown(&documented_sample(), Locale::En);
        let description = doc.find("Plays one track at a time.").expect("description");
        assert!(description < doc.find("```mermaid").unwrap(), "{doc}");
        assert!(description > doc.find("### Machine: PlayerState").unwrap(), "{doc}");
    }

    /// A cell is one line, so a longer description is repeated in full below
    /// the table rather than being truncated away.
    #[test]
    fn markdown_restates_a_multi_paragraph_description_in_full() {
        let doc = markdown(&documented_sample(), Locale::En);
        assert!(doc.contains("##### Stopped"), "{doc}");
        assert!(doc.contains("The track stays loaded, so\nplay resumes it."), "{doc}");
        // The single-paragraph state needs no section of its own.
        assert!(!doc.contains("##### Playing"), "{doc}");
    }

    /// A marker on the state enum describes the whole region, so it is stated
    /// beside the machine's description rather than in the per-state table.
    #[test]
    fn markdown_states_markers_declared_on_the_machine() {
        let mut project = sample();
        let machine = &mut project.cores[0].machines[0];
        machine.markers = vec![Marker::Deprecated];
        machine.tags = vec!["legacy".into()];

        let doc = markdown(&project, Locale::En);
        assert!(doc.contains("**Markers:** deprecated"), "{doc}");
        assert!(doc.contains("**Tags:** `legacy`"), "{doc}");
        // Region-level markers alone must not conjure a states table.
        assert!(!doc.contains("#### States"), "{doc}");

        let pt = markdown(&project, Locale::PtBr);
        assert!(pt.contains("**Marcadores:** descontinuado"), "{pt}");
        assert!(pt.contains("**Etiquetas:** `legacy`"), "{pt}");
    }

    #[test]
    fn markdown_omits_the_states_table_when_nothing_is_documented() {
        let doc = markdown(&sample(), Locale::En);
        assert!(!doc.contains("#### States"), "{doc}");
        assert!(!doc.contains("| State |"), "{doc}");
    }

    #[test]
    fn markdown_escapes_pipes_and_newlines_in_a_description() {
        let mut project = sample();
        project.cores[0].machines[0].states[1].doc =
            Some("Either a | or a\nwrapped line.".into());
        let doc = markdown(&project, Locale::En);
        assert!(doc.contains("| Playing | final | Either a \\| or a wrapped line. |"), "{doc}");
    }

    /// A cell is prose, not a literal: the same backticks that render as code
    /// in a documentation block have to render as code in the table too.
    #[test]
    fn markdown_keeps_author_backticks_live_in_a_description() {
        let mut project = sample();
        project.cores[0].machines[0].states[1].doc =
            Some("`progress` is how far along the bar it is.".into());
        let doc = markdown(&project, Locale::En);
        assert!(
            doc.contains("| Playing | final | `progress` is how far along the bar it is. |"),
            "{doc}"
        );
        assert!(!doc.contains("\\`"), "backtick escaped into the reader's view: {doc}");
    }

    /// A cell is one row: the pipe still has to be escaped inside a code span,
    /// or the code span opens a column.
    #[test]
    fn markdown_escapes_a_pipe_inside_an_author_code_span() {
        let mut project = sample();
        project.cores[0].machines[0].states[1].doc = Some("Either `a | b` or nothing.".into());
        let doc = markdown(&project, Locale::En);
        assert!(doc.contains("| Playing | final | Either `a \\| b` or nothing. |"), "{doc}");
    }

    #[test]
    fn markdown_localizes_the_states_table_but_not_the_authors_text() {
        let doc = markdown(&documented_sample(), Locale::PtBr);
        assert!(doc.contains("#### Estados"), "{doc}");
        assert!(
            doc.contains("| Estado | Papel | Descrição | Marcadores | Etiquetas |"),
            "{doc}"
        );
        // The derived roles are our vocabulary too, so they translate.
        assert!(doc.contains("| Orphan | inicial, final |"), "{doc}");
        assert!(doc.contains("falha, descontinuado"), "{doc}");
        // The author's prose and their tag names are data, not prose of ours.
        assert!(doc.contains("Audio is reaching the speakers."), "{doc}");
        assert!(doc.contains("`idle-ish`"), "{doc}");
        assert!(!doc.contains("| State |"), "English label leaked: {doc}");
        assert!(!doc.contains("deprecated |"), "English marker leaked: {doc}");
    }

    /// The honesty rule as an assertion: switching locale must not touch a
    /// single character the analyzed application wrote.
    #[test]
    fn author_prose_is_identical_in_every_locale() {
        let project = documented_sample();
        for locale in Locale::ALL {
            let doc = markdown(&project, locale);
            assert!(doc.contains("Plays one track at a time."), "{locale}");
            assert!(doc.contains("Audio is reaching the speakers."), "{locale}");
            assert!(
                doc.contains("The track stays loaded, so\nplay resumes it."),
                "{locale}"
            );
        }
    }

    #[test]
    fn mermaid_renders_composite_states_nested() {
        let project = Project {
            project: "S".into(),
            cores: vec![Core {
                name: "C".into(),
                machines: vec![Machine {
                    name: "State".into(),
                    states: vec![
                        "Idle".into(),
                        "Active/Loading".into(),
                        "Active/Ready".into(),
                    ],
                    transitions: vec![Transition {
                        from: State("Idle".into()),
                        event: Event("Start".into()),
                        to: State("Active/Loading".into()),
                        effects: vec![],
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            }],
        };
        let body = &mermaid_diagrams(&project, Locale::En)[0].mermaid;
        assert!(body.contains("state Active {"), "{body}");
        assert!(body.contains("state \"Loading\" as Active_Loading"), "{body}");
        assert!(body.contains("Idle --> Active_Loading: Start"), "{body}");
    }

    #[test]
    fn mermaid_renders_transitions_wildcards_and_orphans() {
        let diagrams = mermaid_diagrams(&sample(), Locale::En);
        assert_eq!(diagrams.len(), 1);
        assert_eq!(diagrams[0].core, "Player");
        assert_eq!(diagrams[0].machine, "PlayerState");

        let body = &diagrams[0].mermaid;
        assert!(body.starts_with("stateDiagram-v2"), "{body}");
        assert!(body.contains("Stopped --> Playing: Play"), "{body}");
        assert!(body.contains("state \"any state\" as any_state"), "{body}");
        assert!(body.contains("any_state --> Stopped: Reset"), "{body}");
        assert!(body.contains("\n    Orphan"), "orphan state must appear: {body}");
    }

    #[test]
    fn markdown_embeds_diagrams_and_transition_tables() {
        let doc = markdown(&sample(), Locale::En);
        assert!(doc.contains("# Sample"), "{doc}");
        assert!(doc.contains("## Core: Player"), "{doc}");
        assert!(doc.contains("### Machine: PlayerState"), "{doc}");
        assert!(doc.contains("```mermaid\nstateDiagram-v2"), "{doc}");
        assert!(doc.contains("| Stopped | `Play` | Playing | `Render`, `Audio::Start` |"), "{doc}");
        assert!(doc.contains("| *any* | `Reset` | Stopped | — |"), "{doc}");
    }

    #[test]
    fn markdown_localizes_prose_and_leaves_identifiers_alone() {
        let doc = markdown(&sample(), Locale::PtBr);
        assert!(doc.contains("## Núcleo: Player"), "{doc}");
        assert!(doc.contains("### Máquina: PlayerState"), "{doc}");
        assert!(doc.contains("| De | Evento | Para | Efeitos |"), "{doc}");
        assert!(doc.contains("| *qualquer* | `Reset` | Stopped | — |"), "{doc}");
        // The project title and every identifier are data, not prose.
        assert!(doc.contains("# Sample"), "{doc}");
        assert!(
            doc.contains("| Stopped | `Play` | Playing | `Render`, `Audio::Start` |"),
            "{doc}"
        );
        assert!(!doc.contains("## Core:"), "English label leaked: {doc}");
    }

    /// `sample()` with what the source declares around its requests.
    fn requesting_sample() -> Project {
        let mut project = sample();
        project.cores[0].machines[0].transitions[0].effects = vec![
            Effect::bare("Render"),
            Effect {
                name: "Audio::Start".into(),
                capability: Some("Audio".into()),
                resolves_with: vec![Event("Started".into()), Event("Failed".into())],
                conditional: false,
            },
            Effect {
                name: "Http::Report".into(),
                capability: Some("Http".into()),
                resolves_with: vec![
                    Event("Reported".into()),
                    Event("ReportFailed".into()),
                    Event("Retried".into()),
                    Event("GaveUp".into()),
                ],
                conditional: true,
            },
        ];
        project
    }

    #[test]
    fn transition_labels_carry_the_effects_statechart_style() {
        let body = &mermaid_diagrams(&requesting_sample(), Locale::En)[0].mermaid;
        // `event / action`, with a conditional request marked as such. The
        // callback events stay out of the diagram.
        assert!(
            body.contains("Stopped --> Playing: Play / Render, Audio::Start, Http::Report?"),
            "{body}"
        );
        assert!(!body.contains("Started"), "callbacks belong in the tables: {body}");
        // A transition that requests nothing keeps a bare event label.
        assert!(body.contains("any_state --> Stopped: Reset"), "{body}");
    }

    #[test]
    fn markdown_shows_what_each_request_answers_with() {
        let doc = markdown(&requesting_sample(), Locale::En);
        assert!(
            doc.contains(
                "| Stopped | `Play` | Playing | `Render`, `Audio::Start` → `Started`, `Failed`, \
                 `Http::Report` → `Reported`, `ReportFailed`, `Retried`, +1 more (conditional) |"
            ),
            "{doc}"
        );

        // One row per capability, with every operation and every answer.
        assert!(doc.contains("### Capabilities"), "{doc}");
        assert!(
            doc.contains("| Capability | Operations | Answers with |"),
            "{doc}"
        );
        assert!(
            doc.contains("| `Audio` | `Audio::Start` | `Failed`, `Started` |"),
            "{doc}"
        );
        assert!(
            doc.contains(
                "| `Http` | `Http::Report` | `GaveUp`, `ReportFailed`, `Reported`, `Retried` |"
            ),
            "{doc}"
        );
    }

    #[test]
    fn markdown_localizes_the_capabilities_table() {
        let doc = markdown(&requesting_sample(), Locale::PtBr);
        assert!(doc.contains("### Capacidades"), "{doc}");
        assert!(
            doc.contains("| Capacidade | Operações | Responde com |"),
            "{doc}"
        );
        assert!(doc.contains("+1 outros (condicional)"), "{doc}");
        assert!(!doc.contains("conditional"), "English label leaked: {doc}");
    }

    /// A core whose requests show no capability emits exactly what it emitted
    /// before capabilities existed.
    #[test]
    fn a_core_with_no_capability_gets_no_capabilities_table() {
        let doc = markdown(&sample(), Locale::En);
        assert!(!doc.contains("### Capabilities"), "{doc}");
    }

    #[test]
    fn mermaid_localizes_the_wildcard_label_but_not_its_node_id() {
        let body = &mermaid_diagrams(&sample(), Locale::PtBr)[0].mermaid;
        assert!(body.contains("state \"qualquer estado\" as any_state"), "{body}");
        // The id is diagram syntax: the transition line must still resolve.
        assert!(body.contains("any_state --> Stopped: Reset"), "{body}");
        assert!(body.starts_with("stateDiagram-v2"), "{body}");
    }
}
