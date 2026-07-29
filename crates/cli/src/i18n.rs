//! The CLI's own message catalog.
//!
//! Covers what `crux-analyzer` writes to stdout/stderr at runtime. Clap's
//! `--help` output is **not** localized: the derive macro takes it from doc
//! comments, which are baked in at compile time and resolved before
//! `--locale` can be read. See `docs/i18n.md`.

use std::path::Path;

use crux_analyzer_i18n::Locale;

/// Localized diagnostic prefixes and status lines.
pub struct Messages(Locale);

impl Messages {
    pub fn new(locale: Locale) -> Self {
        Messages(locale)
    }

    pub fn locale(&self) -> Locale {
        self.0
    }

    /// Prefix for a fatal message (`error: ...`).
    pub fn error_prefix(&self) -> &'static str {
        match self.0 {
            Locale::En => "error",
            Locale::PtBr => "erro",
        }
    }

    /// Prefix for a diagnostic (`warning: ...`).
    pub fn warning_prefix(&self) -> &'static str {
        match self.0 {
            Locale::En => "warning",
            Locale::PtBr => "aviso",
        }
    }

    pub fn failed_to_write(&self, path: &Path) -> String {
        let path = path.display();
        match self.0 {
            Locale::En => format!("failed to write {path}"),
            Locale::PtBr => format!("falha ao escrever {path}"),
        }
    }

    /// The post-write summary: `wrote X (2 cores, 1 warning)`.
    pub fn wrote_summary(&self, path: &Path, cores: usize, warnings: usize) -> String {
        let path = path.display();
        let cores = self.count(cores, Noun::Core);
        let warnings = self.count(warnings, Noun::Warning);
        match self.0 {
            Locale::En => format!("wrote {path} ({cores}, {warnings})"),
            Locale::PtBr => format!("escrito {path} ({cores}, {warnings})"),
        }
    }

    pub fn failed_to_create_watcher(&self) -> &'static str {
        match self.0 {
            Locale::En => "failed to create file watcher",
            Locale::PtBr => "falha ao criar o observador de arquivos",
        }
    }

    pub fn failed_to_watch(&self, path: &Path) -> String {
        let path = path.display();
        match self.0 {
            Locale::En => format!("failed to watch {path}"),
            Locale::PtBr => format!("falha ao observar {path}"),
        }
    }

    pub fn watching(&self, path: &Path) -> String {
        let path = path.display();
        match self.0 {
            Locale::En => format!("watching {path} — Ctrl-C to stop"),
            Locale::PtBr => format!("observando {path} — Ctrl-C para parar"),
        }
    }

    pub fn change_detected(&self) -> &'static str {
        match self.0 {
            Locale::En => "change detected, regenerating…",
            Locale::PtBr => "alteração detectada, regenerando…",
        }
    }

    /// `n` with the correctly inflected noun.
    ///
    /// Both locales inflect on `n != 1`, so one rule covers them; a locale
    /// with different plural categories would need its own arm here.
    fn count(&self, n: usize, noun: Noun) -> String {
        let plural = n != 1;
        let word = match (self.0, noun, plural) {
            (Locale::En, Noun::Core, false) => "core",
            (Locale::En, Noun::Core, true) => "cores",
            (Locale::En, Noun::Warning, false) => "warning",
            (Locale::En, Noun::Warning, true) => "warnings",
            (Locale::PtBr, Noun::Core, false) => "núcleo",
            (Locale::PtBr, Noun::Core, true) => "núcleos",
            (Locale::PtBr, Noun::Warning, false) => "aviso",
            (Locale::PtBr, Noun::Warning, true) => "avisos",
        };
        format!("{n} {word}")
    }
}

#[derive(Clone, Copy)]
enum Noun {
    Core,
    Warning,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_inflect_on_not_one() {
        let en = Messages::new(Locale::En);
        assert_eq!(en.count(0, Noun::Core), "0 cores");
        assert_eq!(en.count(1, Noun::Core), "1 core");
        assert_eq!(en.count(2, Noun::Warning), "2 warnings");

        let pt = Messages::new(Locale::PtBr);
        assert_eq!(pt.count(1, Noun::Core), "1 núcleo");
        assert_eq!(pt.count(3, Noun::Core), "3 núcleos");
        assert_eq!(pt.count(1, Noun::Warning), "1 aviso");
    }

    #[test]
    fn summary_reads_naturally_in_both_locales() {
        let path = Path::new("model.json");
        assert_eq!(
            Messages::new(Locale::En).wrote_summary(path, 1, 0),
            "wrote model.json (1 core, 0 warnings)"
        );
        assert_eq!(
            Messages::new(Locale::PtBr).wrote_summary(path, 2, 1),
            "escrito model.json (2 núcleos, 1 aviso)"
        );
    }
}
