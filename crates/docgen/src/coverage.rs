//! How much of a model carries authored documentation.
//!
//! A tally, not a document: the numbers are computed here and rendered by
//! whoever asked for them (the CLI, with its own message catalog). Keeping the
//! two apart is what makes the counting testable without asserting on prose.
//!
//! "Documented" means **the state has a description**. A state carrying only a
//! marker or a tag is classified, not explained, so it does not count — the
//! point of the measure is prose a reader can learn something from.

use crux_analyzer_model::{Machine, Project, StateDecl};

/// A documented-out-of-total tally.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Coverage {
    pub documented: usize,
    pub total: usize,
}

impl Coverage {
    /// The share documented, rounded to a whole percent for display.
    ///
    /// Nothing to document counts as complete: an empty machine is not a
    /// documentation failure.
    pub fn percent(self) -> u32 {
        if self.total == 0 {
            return 100;
        }
        // Round half up, in integer arithmetic.
        ((self.documented * 200 + self.total) / (self.total * 2)) as u32
    }

    /// Whether this tally meets a required percentage.
    ///
    /// Compared exactly, not through [`percent`](Self::percent): 2 of 3 states
    /// displays as 67% but must not satisfy `--min 67`, and rounding it first
    /// would let it.
    pub fn meets(self, min: u8) -> bool {
        if self.total == 0 {
            return true;
        }
        self.documented * 100 >= min as usize * self.total
    }

    pub fn missing(self) -> usize {
        self.total - self.documented
    }

    fn add(&mut self, other: Coverage) {
        self.documented += other.documented;
        self.total += other.total;
    }
}

/// One machine's tally, with the names needed to report it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineCoverage {
    pub core: String,
    pub machine: String,
    /// Whether the state enum itself carries a description.
    pub machine_documented: bool,
    pub states: Coverage,
    /// Names of the states with no description, in declaration order.
    pub undocumented: Vec<String>,
}

/// A whole project's tally: one entry per machine, plus the total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCoverage {
    pub machines: Vec<MachineCoverage>,
    pub states: Coverage,
    /// Machines whose own state enum carries no description.
    pub machines_documented: Coverage,
}

/// Measures how much of `project` is documented.
pub fn coverage(project: &Project) -> ProjectCoverage {
    let mut machines = Vec::new();
    let mut states = Coverage::default();
    let mut machines_documented = Coverage::default();

    for core in &project.cores {
        for machine in &core.machines {
            let entry = machine_coverage(&core.name, machine);
            states.add(entry.states);
            machines_documented.add(Coverage {
                documented: usize::from(entry.machine_documented),
                total: 1,
            });
            machines.push(entry);
        }
    }

    ProjectCoverage {
        machines,
        states,
        machines_documented,
    }
}

fn machine_coverage(core: &str, machine: &Machine) -> MachineCoverage {
    let documented = machine.states.iter().filter(|s| is_described(s)).count();
    MachineCoverage {
        core: core.to_string(),
        machine: machine.name.clone(),
        machine_documented: machine.doc.is_some(),
        states: Coverage {
            documented,
            total: machine.states.len(),
        },
        undocumented: machine
            .states
            .iter()
            .filter(|s| !is_described(s))
            .map(|s| s.name.clone())
            .collect(),
    }
}

/// A description, specifically — a marker or a tag classifies a state without
/// explaining it.
fn is_described(state: &StateDecl) -> bool {
    state.doc.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crux_analyzer_model::{Core, Marker};

    fn machine(name: &str, doc: Option<&str>, states: Vec<StateDecl>) -> Machine {
        Machine {
            name: name.into(),
            doc: doc.map(str::to_string),
            states,
            ..Default::default()
        }
    }

    fn described(name: &str) -> StateDecl {
        StateDecl {
            name: name.into(),
            doc: Some("Something true about it.".into()),
            ..Default::default()
        }
    }

    fn project(machines: Vec<Machine>) -> Project {
        Project {
            project: "P".into(),
            cores: vec![Core {
                name: "C".into(),
                machines,
                ..Default::default()
            }],
        }
    }

    #[test]
    fn counts_described_states_per_machine() {
        let report = coverage(&project(vec![machine(
            "M",
            Some("The region."),
            vec![described("Idle"), StateDecl::bare("Running")],
        )]));

        assert_eq!(report.machines.len(), 1);
        let entry = &report.machines[0];
        assert_eq!(entry.core, "C");
        assert_eq!(entry.machine, "M");
        assert!(entry.machine_documented);
        assert_eq!(entry.states, Coverage { documented: 1, total: 2 });
        assert_eq!(entry.undocumented, ["Running"]);
    }

    /// The measure is about prose: classification is not explanation.
    #[test]
    fn a_marker_or_tag_alone_does_not_count_as_documented() {
        let classified = StateDecl {
            name: "Failed".into(),
            markers: vec![Marker::Failure],
            tags: vec!["retryable".into()],
            ..Default::default()
        };
        let report = coverage(&project(vec![machine("M", None, vec![classified])]));
        assert_eq!(report.states, Coverage { documented: 0, total: 1 });
        assert_eq!(report.machines[0].undocumented, ["Failed"]);
    }

    #[test]
    fn totals_across_machines_and_cores() {
        let report = coverage(&Project {
            project: "P".into(),
            cores: vec![
                Core {
                    name: "A".into(),
                    machines: vec![machine("M1", Some("doc"), vec![described("X")])],
                    ..Default::default()
                },
                Core {
                    name: "B".into(),
                    machines: vec![machine(
                        "M2",
                        None,
                        vec![described("Y"), StateDecl::bare("Z")],
                    )],
                    ..Default::default()
                },
            ],
        });
        assert_eq!(report.states, Coverage { documented: 2, total: 3 });
        assert_eq!(
            report.machines_documented,
            Coverage { documented: 1, total: 2 }
        );
        assert_eq!(report.machines.len(), 2);
        assert_eq!(report.machines[1].core, "B");
    }

    #[test]
    fn percent_rounds_half_up() {
        assert_eq!(Coverage { documented: 2, total: 3 }.percent(), 67);
        assert_eq!(Coverage { documented: 1, total: 3 }.percent(), 33);
        assert_eq!(Coverage { documented: 1, total: 2 }.percent(), 50);
        assert_eq!(Coverage { documented: 3, total: 3 }.percent(), 100);
        assert_eq!(Coverage { documented: 0, total: 4 }.percent(), 0);
    }

    /// Nothing to document is not a failure — an empty project must not fail CI.
    #[test]
    fn an_empty_tally_is_complete() {
        let empty = Coverage::default();
        assert_eq!(empty.percent(), 100);
        assert!(empty.meets(100));
        assert_eq!(empty.missing(), 0);
    }

    /// `meets` compares exactly, so a displayed 67% does not satisfy `--min 67`.
    #[test]
    fn meets_does_not_inherit_display_rounding() {
        let two_of_three = Coverage { documented: 2, total: 3 };
        assert_eq!(two_of_three.percent(), 67);
        assert!(!two_of_three.meets(67));
        assert!(two_of_three.meets(66));
        assert!(Coverage { documented: 3, total: 3 }.meets(100));
    }
}
