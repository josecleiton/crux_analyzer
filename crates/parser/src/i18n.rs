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
//!
//! They are, however, *sanitized*: an identifier or a doc-comment line is
//! attacker-controlled text on its way to a terminal, and an ANSI escape
//! sequence embedded in one would rewrite the operator's screen. [`sanitize`]
//! runs over every rendered diagnostic — see `docs/security.md`.

use crux_analyzer_i18n::Locale;

use crate::{ParseError, WarningKind};

/// Strips control characters from text bound for a terminal.
///
/// Applied to whole rendered messages rather than to each interpolation, so a
/// variant added later cannot forget it. Newlines and tabs go too: a diagnostic
/// is one line, and a doc comment that smuggles in a newline would otherwise
/// forge a second one.
pub(crate) fn sanitize(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_control() { '\u{fffd}' } else { c })
        .collect()
}

impl WarningKind {
    /// The message for this diagnostic in `locale`.
    pub fn message(&self, locale: Locale) -> String {
        let raw = match locale {
            Locale::En => self.message_en(),
            Locale::PtBr => self.message_pt_br(),
        };
        sanitize(&raw)
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
            WarningKind::UnresolvedEffectCallback => {
                "effect callback not resolved: the event this request is answered \
                 with is not named at the call site"
                    .to_string()
            }
            WarningKind::UnknownAnnotation { annotation } => format!(
                "unrecognized annotation `{annotation}`: not one of @failure, \
                 @deprecated, @tag <name>"
            ),
            WarningKind::AnalysisTruncated { core, limit } => format!(
                "core {core}: analysis stopped at the {limit} limit — the model \
                 may be incomplete. Raise it if this source is trusted."
            ),
            WarningKind::FileTooLarge { size, max } => {
                format!("file skipped: {size} bytes exceeds the {max}-byte limit")
            }
            WarningKind::InputTooLarge { max } => format!(
                "remaining files skipped: the run reached the {max}-byte total \
                 source limit"
            ),
            WarningKind::SourceUnreadable { reason } => {
                format!("path skipped: {reason}")
            }
            WarningKind::NotARegularFile => {
                "path skipped: not a regular file (symlink, device or FIFO)".to_string()
            }
            WarningKind::NestingTooDeep { max } => {
                format!("file skipped: brackets nest deeper than {max} levels")
            }
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
            WarningKind::UnresolvedEffectCallback => {
                "callback de efeito não resolvido: o evento que responde a esta \
                 solicitação não é nomeado no local da chamada"
                    .to_string()
            }
            WarningKind::UnknownAnnotation { annotation } => format!(
                "anotação `{annotation}` não reconhecida: não é @failure, \
                 @deprecated nem @tag <nome>"
            ),
            WarningKind::AnalysisTruncated { core, limit } => format!(
                "núcleo {core}: a análise parou no limite de {limit} — o modelo \
                 pode estar incompleto. Aumente o limite se esta fonte é confiável."
            ),
            WarningKind::FileTooLarge { size, max } => format!(
                "arquivo ignorado: {size} bytes excede o limite de {max} bytes"
            ),
            WarningKind::InputTooLarge { max } => format!(
                "arquivos restantes ignorados: a execução alcançou o limite total \
                 de {max} bytes de código-fonte"
            ),
            WarningKind::SourceUnreadable { reason } => {
                format!("caminho ignorado: {reason}")
            }
            WarningKind::NotARegularFile => {
                "caminho ignorado: não é um arquivo regular (link simbólico, \
                 dispositivo ou FIFO)"
                    .to_string()
            }
            WarningKind::NestingTooDeep { max } => format!(
                "arquivo ignorado: os delimitadores aninham mais de {max} níveis"
            ),
        }
    }
}

impl ParseError {
    /// The message for this failure in `locale`. Sanitized like a warning: the
    /// path and `syn`'s prose both quote the analyzed source.
    pub fn message(&self, locale: Locale) -> String {
        sanitize(&self.raw_message(locale))
    }

    fn raw_message(&self, locale: Locale) -> String {
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
            WarningKind::UnknownAnnotation {
                annotation: "@failur".into(),
            },
            WarningKind::AnalysisTruncated {
                core: "Recorder".into(),
                limit: "max-steps".into(),
            },
            WarningKind::FileTooLarge {
                size: 9_000_000,
                max: 2_097_152,
            },
            WarningKind::InputTooLarge { max: 268_435_456 },
            WarningKind::SourceUnreadable {
                reason: "permission denied".into(),
            },
            WarningKind::NotARegularFile,
            WarningKind::NestingTooDeep { max: 192 },
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
                "unresolvable-source",
                "unknown-annotation",
                "analysis-truncated",
                "file-too-large",
                "input-too-large",
                "source-unreadable",
                "not-a-regular-file",
                "nesting-too-deep"
            ]
        );
    }

    /// A doc comment is attacker-controlled text and a diagnostic goes to a
    /// terminal: an ANSI escape smuggled through an annotation must not survive
    /// into the rendered message. See `docs/security.md`.
    #[test]
    fn diagnostics_strip_control_characters() {
        let kind = WarningKind::UnknownAnnotation {
            annotation: "@x\u{1b}[31mred\u{7}\nfake line".into(),
        };
        for locale in Locale::ALL {
            let message = kind.message(locale);
            assert!(
                !message.chars().any(char::is_control),
                "{locale}: {message:?} still carries a control character"
            );
            // The visible text survives — only the control bytes are replaced.
            assert!(message.contains("red"), "{locale}");
        }
    }

    #[test]
    fn rendered_warnings_sanitize_the_path_too() {
        let warning = crate::Warning {
            file: std::path::PathBuf::from("src/\u{1b}[2Kevil.rs"),
            line: 3,
            kind: WarningKind::NotARegularFile,
        };
        for locale in Locale::ALL {
            let rendered = warning.render(locale);
            assert!(!rendered.chars().any(char::is_control), "{locale}");
        }
    }
}
