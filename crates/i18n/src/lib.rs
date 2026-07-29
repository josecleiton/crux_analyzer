//! The locale every crux_analyzer crate agrees on.
//!
//! This crate deliberately holds **only** [`Locale`] and its detection — no
//! message catalogs. Each crate owns the catalog for its own strings
//! (`crux_analyzer_parser::i18n` for warnings, `crux_analyzer_docgen::Labels`
//! for generated documents, `crux-analyzer`'s own module for CLI output), so
//! no crate has to know another's message set and the dependency graph stays a
//! clean fan-in to this one.
//!
//! English is the **source locale**: keys and fallbacks are authored in
//! English, and anything unlocalized renders in English rather than failing.

use std::fmt;
use std::str::FromStr;

/// A supported output language.
///
/// Only text written *by* crux_analyzer is localized. Identifiers read out of
/// the analyzed application (core, machine, state, event and effect names) are
/// data and are never translated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Locale {
    /// English — the source locale and the fallback.
    #[default]
    En,
    /// Brazilian Portuguese.
    PtBr,
}

impl Locale {
    /// Every supported locale, in declaration order.
    pub const ALL: [Locale; 2] = [Locale::En, Locale::PtBr];

    /// The canonical BCP 47 tag (`en`, `pt-BR`).
    pub fn tag(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::PtBr => "pt-BR",
        }
    }

    /// The locale's own name for itself, for language pickers.
    pub fn endonym(self) -> &'static str {
        match self {
            Locale::En => "English",
            Locale::PtBr => "Português (Brasil)",
        }
    }

    /// Resolves the locale from the environment.
    ///
    /// Precedence: `CRUX_ANALYZER_LOCALE`, then the POSIX chain `LC_ALL` →
    /// `LC_MESSAGES` → `LANG`. Unset, empty, `C`/`POSIX` and unrecognized
    /// values all fall back to [`Locale::En`], so the CLI never fails on a
    /// locale it does not know.
    pub fn from_env() -> Locale {
        ["CRUX_ANALYZER_LOCALE", "LC_ALL", "LC_MESSAGES", "LANG"]
            .iter()
            .filter_map(|key| std::env::var(key).ok())
            .find_map(|value| Locale::from_posix(&value))
            .unwrap_or_default()
    }

    /// Parses one POSIX locale value (`pt_BR.UTF-8`, `pt-br`, `C`, ...).
    ///
    /// Returns `None` when the value carries no usable language, so the caller
    /// can keep walking the precedence chain instead of stopping at a `C`.
    fn from_posix(value: &str) -> Option<Locale> {
        // Strip the codeset and modifier: `pt_BR.UTF-8@dict` -> `pt_BR`.
        let language_region = value
            .split(['.', '@'])
            .next()
            .unwrap_or("")
            .trim()
            .replace('_', "-");
        if language_region.is_empty() {
            return None;
        }
        Locale::from_tag(&language_region)
    }

    /// Matches a BCP 47 tag, accepting a bare language (`pt`) and any region
    /// of it. Portuguese of any region maps to pt-BR — it is the only
    /// Portuguese catalog that exists.
    fn from_tag(tag: &str) -> Option<Locale> {
        let lowered = tag.to_ascii_lowercase();
        let language = lowered.split('-').next().unwrap_or("");
        match language {
            "en" => Some(Locale::En),
            "pt" => Some(Locale::PtBr),
            _ => None,
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

/// Returned when an explicit `--locale` value names a locale we do not ship.
///
/// Unlike environment detection, an explicit request for a missing locale is
/// an error: silently falling back to English would ignore the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownLocale(pub String);

impl fmt::Display for UnknownLocale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let supported = Locale::ALL
            .iter()
            .map(|l| l.tag())
            .collect::<Vec<_>>()
            .join(", ");
        write!(f, "unknown locale `{}` (supported: {supported})", self.0)
    }
}

impl std::error::Error for UnknownLocale {}

impl FromStr for Locale {
    type Err = UnknownLocale;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Locale::from_tag(value.trim()).ok_or_else(|| UnknownLocale(value.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_tags() {
        assert_eq!("en".parse(), Ok(Locale::En));
        assert_eq!("pt-BR".parse(), Ok(Locale::PtBr));
    }

    #[test]
    fn parsing_is_case_and_region_insensitive() {
        assert_eq!("PT-br".parse(), Ok(Locale::PtBr));
        assert_eq!("pt".parse(), Ok(Locale::PtBr));
        assert_eq!("pt-PT".parse(), Ok(Locale::PtBr));
        assert_eq!("en-GB".parse(), Ok(Locale::En));
    }

    #[test]
    fn explicit_unknown_locale_is_an_error() {
        let err = "fr".parse::<Locale>().unwrap_err();
        assert_eq!(err, UnknownLocale("fr".to_string()));
        assert!(err.to_string().contains("supported: en, pt-BR"), "{err}");
    }

    #[test]
    fn posix_values_drop_codeset_and_modifier() {
        assert_eq!(Locale::from_posix("pt_BR.UTF-8"), Some(Locale::PtBr));
        assert_eq!(Locale::from_posix("en_US.UTF-8@euro"), Some(Locale::En));
    }

    #[test]
    fn posix_values_without_a_language_are_skipped() {
        // `None` (not `En`) so the caller keeps walking the precedence chain.
        assert_eq!(Locale::from_posix("C"), None);
        assert_eq!(Locale::from_posix("POSIX"), None);
        assert_eq!(Locale::from_posix(""), None);
        assert_eq!(Locale::from_posix(".UTF-8"), None);
    }

    #[test]
    fn english_is_the_default() {
        assert_eq!(Locale::default(), Locale::En);
        assert_eq!(Locale::En.tag(), "en");
    }
}
