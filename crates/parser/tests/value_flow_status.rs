//! Integration test over the `value_flow_status` fixture: state machines whose
//! field the core only ever writes by value flow, detected through model
//! reachability. See `docs/roadmap.md` §6.

use std::path::Path;

use crux_analyzer_parser::parse_project;

fn parse() -> crux_analyzer_parser::ParseOutcome {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/value_flow_status");
    parse_project(&fixture, "Value Flow Status").expect("fixture must parse")
}

#[test]
fn detects_a_machine_the_core_never_writes_as_a_literal_variant() {
    let outcome = parse();
    let core = &outcome.project.cores[0];

    let names: Vec<&str> = core.machines.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(
        names,
        ["JobStatus"],
        "the per-entity status is the only machine here"
    );

    let machine = &core.machines[0];
    assert_eq!(
        machine.states.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["Pending", "Running", "Done", "Deferred", "Unavailable"]
    );
    assert!(
        machine.doc.as_deref().is_some_and(|doc| !doc.is_empty()),
        "the enum's own doc comment reaches the machine"
    );
}

/// The exclusion the literal-assignment rule used to provide for free. A mirror
/// enum is assigned *and* dispatched on, so only reachability keeps it out.
#[test]
fn a_view_mirror_enum_is_not_a_machine() {
    let outcome = parse();
    let core = &outcome.project.cores[0];
    assert!(
        !core.machines.iter().any(|m| m.name == "ViewStatus"),
        "a mirror enum the model never holds must not become a machine: {:#?}",
        core.machines.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
}

#[test]
fn a_payload_write_yields_a_wildcard_target() {
    let outcome = parse();
    let machine = &outcome.project.cores[0].machines[0];
    let triples: Vec<(&str, &str, &str)> = machine
        .transitions
        .iter()
        .map(|t| (t.from.0.as_str(), t.event.0.as_str(), t.to.0.as_str()))
        .collect();
    assert_eq!(triples, [("*", "StatusReported", "*")]);
}

/// The carry-over write is a clone guarded by a `matches!` on *another* entry's
/// same-named field. Source constraints are keyed by field name, so the two
/// guards intersect to nothing and the target cannot be placed. It must be
/// reported rather than dropped in silence — resolving the aliasing is the next
/// increment, and this is the test that should change when it lands.
#[test]
fn a_contradictory_source_constraint_warns_instead_of_vanishing() {
    let outcome = parse();
    let codes: Vec<&str> = outcome.warnings.iter().map(|w| w.kind.code()).collect();
    assert_eq!(codes, ["unresolvable-source", "unresolvable-source"]);
}
