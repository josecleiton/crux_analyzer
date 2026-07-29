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
