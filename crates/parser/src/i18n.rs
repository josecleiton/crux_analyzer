//! Localized renderings of the parser's diagnostics.
//!
//! Diagnostics are constructed as data ([`WarningKind`], [`ParseError`]) and
//! turned into prose only here, at the moment they are shown. That is what
//! makes them localizable at all: the walker that finds a problem is deep in
//! the AST and has no business knowing the caller's language.
//!
//! Identifiers interpolated into these messages (core, machine and state
//! names) come from the analyzed application's source and are never
//! translated.

use crux_analyzer_i18n::Locale;

use crate::{ParseError, WarningKind};

impl WarningKind {
    /// The message for this diagnostic in `locale`.
    pub fn message(&self, locale: Locale) -> String {
        match locale {
            Locale::En => self.message_en(),
            Locale::PtBr => self.message_pt_br(),
        }
    }

    fn message_en(&self) -> String {
        match self {
            WarningKind::NoUpdateMethod { core } => {
                format!("core {core}: no `update` method found")
            }
            WarningKind::DynamicTarget { machine } => format!(
                "transition of `{machine}` dropped: target state is dynamic \
                 (assigned from a runtime value)"
            ),
            WarningKind::UnknownEvent { to } => {
                format!("transition to `{to}` dropped: could not infer the triggering event")
            }
            WarningKind::UnresolvableSource { to } => format!(
                "transition to `{to}` dropped: source-state condition could not \
                 be resolved statically"
            ),
        }
    }

    fn message_pt_br(&self) -> String {
        match self {
            WarningKind::NoUpdateMethod { core } => {
                format!("núcleo {core}: método `update` não encontrado")
            }
            WarningKind::DynamicTarget { machine } => format!(
                "transição de `{machine}` descartada: o estado de destino é dinâmico \
                 (atribuído a partir de um valor definido em tempo de execução)"
            ),
            WarningKind::UnknownEvent { to } => format!(
                "transição para `{to}` descartada: não foi possível inferir o evento \
                 que a dispara"
            ),
            WarningKind::UnresolvableSource { to } => format!(
                "transição para `{to}` descartada: a condição do estado de origem não \
                 pôde ser resolvida estaticamente"
            ),
        }
    }
}

impl ParseError {
    /// The message for this failure in `locale`.
    pub fn message(&self, locale: Locale) -> String {
        match locale {
            Locale::En => match self {
                ParseError::Io(path, err) => format!("failed to read {}: {err}", path.display()),
                ParseError::Syntax(path, err) => {
                    format!("failed to parse {}: {err}", path.display())
                }
                ParseError::NoCoreFound => "no `impl App for ...` block found".to_string(),
            },
            Locale::PtBr => match self {
                ParseError::Io(path, err) => format!("falha ao ler {}: {err}", path.display()),
                ParseError::Syntax(path, err) => {
                    format!("falha ao analisar {}: {err}", path.display())
                }
                ParseError::NoCoreFound => {
                    "nenhum bloco `impl App for ...` encontrado".to_string()
                }
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds() -> Vec<WarningKind> {
        vec![
            WarningKind::NoUpdateMethod {
                core: "Recorder".into(),
            },
            WarningKind::DynamicTarget {
                machine: "RecorderState".into(),
            },
            WarningKind::UnknownEvent { to: "Idle".into() },
            WarningKind::UnresolvableSource {
                to: "Recording".into(),
            },
        ]
    }

    #[test]
    fn every_kind_has_a_message_in_every_locale() {
        for kind in kinds() {
            for locale in Locale::ALL {
                let message = kind.message(locale);
                assert!(!message.is_empty(), "{:?} in {locale}", kind.code());
            }
        }
    }

    #[test]
    fn translations_differ_from_the_source_locale() {
        // Catches a pt-BR arm accidentally left copied from English.
        for kind in kinds() {
            assert_ne!(
                kind.message(Locale::En),
                kind.message(Locale::PtBr),
                "{} is not translated",
                kind.code()
            );
        }
    }

    #[test]
    fn messages_keep_the_analyzed_identifiers_verbatim() {
        let kind = WarningKind::DynamicTarget {
            machine: "RecorderState".into(),
        };
        for locale in Locale::ALL {
            assert!(kind.message(locale).contains("RecorderState"), "{locale}");
        }
    }

    #[test]
    fn codes_are_unique_and_stable() {
        let codes: Vec<&str> = kinds().iter().map(|k| k.code()).collect();
        assert_eq!(
            codes,
            vec![
                "no-update-method",
                "dynamic-target",
                "unknown-event",
                "unresolvable-source"
            ]
        );
    }
}
