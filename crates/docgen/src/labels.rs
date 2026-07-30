//! The prose a generated document is made of, per locale.
//!
//! Only labels live here. Everything a generator emits that is *not* in this
//! struct is either Markdown/Mermaid syntax or data read out of the analyzed
//! application — state, event, effect, machine and core names are Rust
//! identifiers and are never translated.

use crux_analyzer_i18n::Locale;

/// Localized labels for one document.
#[derive(Debug, Clone, Copy)]
pub struct Labels {
    /// Prefix of the per-core section heading (`## Core: Recorder`).
    pub core: &'static str,
    /// Prefix of the per-machine section heading (`### Machine: RecorderState`).
    pub machine: &'static str,
    /// Transition table column: source state.
    pub from: &'static str,
    /// Transition table column: triggering event.
    pub event: &'static str,
    /// Transition table column: target state.
    pub to: &'static str,
    /// Transition table column: requested effects.
    pub effects: &'static str,
    /// Table cell standing in for a wildcard source state (`from: "*"`).
    pub any_source: &'static str,
    /// Display label of the Mermaid wildcard pseudo-state. Its *id* stays
    /// `any_state` in every locale — that is diagram syntax, not prose.
    pub any_state: &'static str,
    /// Table cell for a transition that requests no effects.
    pub no_effects: &'static str,
    /// Qualifier on an effect the transition's own path does not imply —
    /// requested on a branch below it.
    pub conditional: &'static str,
    /// Heading of the per-core capabilities table.
    pub capabilities: &'static str,
    /// Capabilities table column: the capability itself.
    pub capability: &'static str,
    /// Capabilities table column: the operations requested through it.
    pub operations: &'static str,
    /// Capabilities table column: the events those requests are answered with.
    pub answers: &'static str,
    /// Suffix for the answers a transition cell did not list, as `+3 {more}`.
    /// The whole set stays in the capabilities table.
    pub more: &'static str,
    /// States table heading.
    pub states: &'static str,
    /// Heading of the per-core documented-events table. The `effects` label
    /// doubles as the documented-effects heading.
    pub events: &'static str,
    /// Documented-effects table column: one effect (singular of `effects`).
    pub effect: &'static str,
    /// States table column: the state's name.
    pub state: &'static str,
    /// States table column: the roles derived from the machine's shape and its
    /// `#[default]` declaration. Separate from `markers`, which is what the
    /// author declared.
    pub role: &'static str,
    /// Rendered name of the derived `initial` role — the machine's entry point.
    /// Not a [`Marker`](crux_analyzer_model::Marker) value: no model field
    /// carries this word, so it exists only as prose in a document.
    pub role_initial: &'static str,
    /// Rendered name of the derived `final` role — a state nothing leaves.
    pub role_final: &'static str,
    /// States table column: the description authored in the analyzed source.
    pub description: &'static str,
    /// States table column: markers declared in the source.
    pub markers: &'static str,
    /// States table column: free-form `@tag` values.
    pub tags: &'static str,
    /// Rendered name of the `failure` marker. The marker *value* stays
    /// `failure` in the model — a stable identifier, like `WarningKind::code()`.
    pub marker_failure: &'static str,
    /// Rendered name of the `deprecated` marker.
    pub marker_deprecated: &'static str,
    /// States table cell for an absent description, marker or tag. Separate
    /// from `no_effects`, which would read as a bug in a description column.
    pub no_value: &'static str,
}

impl Labels {
    /// English — the source locale.
    pub const EN: Labels = Labels {
        core: "Core",
        machine: "Machine",
        from: "From",
        event: "Event",
        to: "To",
        effects: "Effects",
        any_source: "*any*",
        any_state: "any state",
        no_effects: "—",
        conditional: "conditional",
        capabilities: "Capabilities",
        capability: "Capability",
        operations: "Operations",
        answers: "Answers with",
        more: "more",
        states: "States",
        events: "Events",
        effect: "Effect",
        state: "State",
        role: "Role",
        role_initial: "initial",
        role_final: "final",
        description: "Description",
        markers: "Markers",
        tags: "Tags",
        marker_failure: "failure",
        marker_deprecated: "deprecated",
        no_value: "—",
    };

    /// Brazilian Portuguese.
    pub const PT_BR: Labels = Labels {
        core: "Núcleo",
        machine: "Máquina",
        from: "De",
        event: "Evento",
        to: "Para",
        effects: "Efeitos",
        any_source: "*qualquer*",
        any_state: "qualquer estado",
        // An em dash is locale-neutral; kept in the struct so a future locale
        // that needs different filler has somewhere to put it.
        no_effects: "—",
        conditional: "condicional",
        capabilities: "Capacidades",
        capability: "Capacidade",
        operations: "Operações",
        answers: "Responde com",
        more: "outros",
        states: "Estados",
        events: "Eventos",
        effect: "Efeito",
        state: "Estado",
        role: "Papel",
        role_initial: "inicial",
        role_final: "final",
        description: "Descrição",
        markers: "Marcadores",
        tags: "Etiquetas",
        // These match the web UI's `badge.failure` / `badge.deprecated` — and
        // the roles above its `badge.initial` / `badge.final` — so a generated
        // document and the app read the same way.
        marker_failure: "falha",
        marker_deprecated: "descontinuado",
        no_value: "—",
    };

    pub fn for_locale(locale: Locale) -> Labels {
        match locale {
            Locale::En => Labels::EN,
            Locale::PtBr => Labels::PT_BR,
        }
    }
}

impl Default for Labels {
    fn default() -> Self {
        Labels::EN
    }
}
