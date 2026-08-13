//! The CLI's own message catalog.
//!
//! Covers what `crux-analyzer` writes to stdout/stderr at runtime. Clap's
//! `--help` output is **not** localized: the derive macro takes it from doc
//! comments, which are baked in at compile time and resolved before
//! `--locale` can be read. See `docs/i18n.md`.

use std::path::Path;

use crux_analyzer_docgen::{Coverage, ProjectCoverage};
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

    /// The post-site summary: `wrote static site to X (2 cores, 1 warning)`.
    pub fn wrote_site_summary(&self, path: &Path, cores: usize, warnings: usize) -> String {
        let path = path.display();
        let cores = self.count(cores, Noun::Core);
        let warnings = self.count(warnings, Noun::Warning);
        match self.0 {
            Locale::En => format!("wrote static site to {path} ({cores}, {warnings})"),
            Locale::PtBr => format!("site estático gerado em {path} ({cores}, {warnings})"),
        }
    }

    pub fn web_assets_missing(&self) -> &'static str {
        match self.0 {
            Locale::En => "web assets are missing — build the web application first (e.g. 'just web-build')",
            Locale::PtBr => "os arquivos da web estão ausentes — construa a aplicação web primeiro (ex: 'just web-build')",
        }
    }

    /// Why `--deny-warnings` failed the run.
    ///
    /// Written out per plural branch rather than interpolating a counted noun:
    /// Portuguese inflects the participle too, so `{n} aviso` + `reportado(s)`
    /// would be the wrong shape in both directions.
    pub fn warnings_denied(&self, n: usize) -> String {
        match (self.0, n != 1) {
            (Locale::En, false) => {
                format!("{n} warning reported and --deny-warnings is set")
            }
            (Locale::En, true) => {
                format!("{n} warnings reported and --deny-warnings is set")
            }
            (Locale::PtBr, false) => {
                format!("{n} aviso reportado e --deny-warnings está ativo")
            }
            (Locale::PtBr, true) => {
                format!("{n} avisos reportados e --deny-warnings está ativo")
            }
        }
    }

    /// Why `--min` failed the run.
    pub fn coverage_below_minimum(&self, actual: u32, min: u8) -> String {
        match self.0 {
            Locale::En => {
                format!("documentation coverage is {actual}%, below the required {min}%")
            }
            Locale::PtBr => {
                format!("a cobertura de documentação é {actual}%, abaixo dos {min}% exigidos")
            }
        }
    }

    /// The full coverage report, one line per machine plus a total.
    ///
    /// Machine and state names are identifiers read out of the analyzed
    /// application, so only the column words and the totals line localize.
    pub fn coverage_report(&self, report: &ProjectCoverage, list: bool) -> String {
        let mut out = String::new();
        for machine in &report.machines {
            out.push_str(&format!(
                "{:<44} {:>3}%  {}\n",
                format!("{} / {}", machine.core, machine.machine),
                machine.states.percent(),
                self.described_of(machine.states),
            ));
            if !machine.machine_documented {
                out.push_str(&format!("  ({})\n", self.machine_undescribed()));
            }
            if list {
                for state in &machine.undocumented {
                    out.push_str(&format!("  - {state}\n"));
                }
            }
        }
        out.push_str(&format!(
            "{:<44} {:>3}%  {}\n",
            self.total(),
            report.states.percent(),
            self.described_of(report.states),
        ));
        out
    }

    /// `3 of 5 states described`. Portuguese inflects both the noun and the
    /// participle on the total, so each branch is written out.
    fn described_of(&self, coverage: Coverage) -> String {
        let (n, total) = (coverage.documented, coverage.total);
        match (self.0, total != 1) {
            (Locale::En, false) => format!("{n} of {total} state described"),
            (Locale::En, true) => format!("{n} of {total} states described"),
            (Locale::PtBr, false) => format!("{n} de {total} estado descrito"),
            (Locale::PtBr, true) => format!("{n} de {total} estados descritos"),
        }
    }

    fn machine_undescribed(&self) -> &'static str {
        match self.0 {
            Locale::En => "the state enum itself has no description",
            Locale::PtBr => "o enum de estado em si não tem descrição",
        }
    }

    fn total(&self) -> &'static str {
        match self.0 {
            Locale::En => "total",
            Locale::PtBr => "total",
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
    use crux_analyzer_docgen::MachineCoverage;

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

    /// Portuguese inflects the participle as well as the noun, which is why
    /// these phrases are not built by interpolating a counted noun.
    #[test]
    fn counted_phrases_agree_in_both_locales() {
        let en = Messages::new(Locale::En);
        let pt = Messages::new(Locale::PtBr);

        assert!(en.warnings_denied(1).starts_with("1 warning reported"));
        assert!(en.warnings_denied(2).starts_with("2 warnings reported"));
        assert!(pt.warnings_denied(1).starts_with("1 aviso reportado e"));
        assert!(pt.warnings_denied(2).starts_with("2 avisos reportados e"));

        let one = Coverage {
            documented: 1,
            total: 1,
        };
        let some = Coverage {
            documented: 1,
            total: 3,
        };
        assert_eq!(en.described_of(one), "1 of 1 state described");
        assert_eq!(en.described_of(some), "1 of 3 states described");
        assert_eq!(pt.described_of(one), "1 de 1 estado descrito");
        assert_eq!(pt.described_of(some), "1 de 3 estados descritos");
    }

    #[test]
    fn the_coverage_report_lists_machines_then_a_total() {
        let report = ProjectCoverage {
            machines: vec![MachineCoverage {
                core: "Recorder".into(),
                machine: "RecorderState".into(),
                machine_documented: false,
                states: Coverage {
                    documented: 1,
                    total: 2,
                },
                undocumented: vec!["Running".into()],
            }],
            states: Coverage {
                documented: 1,
                total: 2,
            },
            machines_documented: Coverage {
                documented: 0,
                total: 1,
            },
        };

        let en = Messages::new(Locale::En).coverage_report(&report, false);
        assert!(en.contains("Recorder / RecorderState"), "{en}");
        assert!(en.contains("50%"), "{en}");
        assert!(
            en.contains("the state enum itself has no description"),
            "{en}"
        );
        assert!(en.contains("total"), "{en}");
        // Without --list the undocumented names stay out.
        assert!(!en.contains("- Running"), "{en}");

        let listed = Messages::new(Locale::En).coverage_report(&report, true);
        assert!(listed.contains("- Running"), "{listed}");

        // Identifiers are data: they read the same in every locale.
        let pt = Messages::new(Locale::PtBr).coverage_report(&report, true);
        assert!(pt.contains("Recorder / RecorderState"), "{pt}");
        assert!(pt.contains("- Running"), "{pt}");
        assert!(pt.contains("1 de 2 estados descritos"), "{pt}");
        assert!(!pt.contains("described"), "English leaked: {pt}");
    }

    #[test]
    fn the_minimum_failure_names_both_numbers() {
        assert_eq!(
            Messages::new(Locale::En).coverage_below_minimum(67, 80),
            "documentation coverage is 67%, below the required 80%"
        );
        assert_eq!(
            Messages::new(Locale::PtBr).coverage_below_minimum(67, 80),
            "a cobertura de documentação é 67%, abaixo dos 80% exigidos"
        );
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
