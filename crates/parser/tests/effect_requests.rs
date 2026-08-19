//! Integration test over the `effect_requests` fixture: which paths in an update
//! body are effect *requests*, and which are payload travelling inside one.
//! See `docs/roadmap.md` §8, P3a and P3b.

use std::path::Path;

use crux_analyzer_parser::parse_project;

fn parse() -> crux_analyzer_parser::ParseOutcome {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/effect_requests");
    parse_project(&fixture, "Effect Requests").expect("fixture must parse")
}

/// Effects per transition, in the order the source requests them.
fn effects_by_event(outcome: &crux_analyzer_parser::ParseOutcome) -> Vec<(String, Vec<String>)> {
    outcome.project.cores[0].machines[0]
        .transitions
        .iter()
        .map(|transition| {
            (
                transition.event.0.clone(),
                transition
                    .effects
                    .iter()
                    .map(|effect| effect.name.clone())
                    .collect(),
            )
        })
        .collect()
}

/// The whole finding in one assertion. Every name here that is *absent* was
/// reported as a request before P3a and P3b: two variants of a payload enum
/// reached at depth 2, one variant of another at depth 3, and an associated
/// function on an operation enum.
#[test]
fn only_what_the_root_wraps_directly_is_a_request() {
    let outcome = parse();
    let requested = effects_by_event(&outcome);
    let requested: Vec<(&str, Vec<&str>)> = requested
        .iter()
        .map(|(event, effects)| {
            (
                event.as_str(),
                effects.iter().map(String::as_str).collect::<Vec<_>>(),
            )
        })
        .collect();

    assert_eq!(
        requested,
        [
            // `Outcome::Landed` fills the request and is not one.
            ("Started", vec!["AudioOperation::Record"]),
            // `Outcome::Refused`, `Domain::Capture` and `AudioOperation::of` are
            // a payload, a payload's payload, and a call. Only the render is a
            // request — and the `Outcome::Landed` that used to appear here as
            // "conditional" came from inside the classifier's own body.
            ("Refused", vec!["Render"]),
            // The two that must survive: an operation the root carries itself,
            // and the bare `render()` that arrives by another path.
            ("Announced", vec!["Effect::Announce", "Render"]),
        ]
    );
}

/// A request the root carries as its own variant has no capability — the same
/// answer `capability_of` gives for a payload enum. Asking only "does it have a
/// capability?" would therefore drop it, which is why the predicate asks for the
/// root by name as well.
#[test]
fn an_operation_the_root_carries_itself_is_still_a_request() {
    let outcome = parse();
    let announce = outcome.project.cores[0].machines[0]
        .transitions
        .iter()
        .flat_map(|transition| &transition.effects)
        .find(|effect| effect.name == "Effect::Announce")
        .expect("the root-carried operation is a request");
    assert!(
        announce.capability.is_none(),
        "nothing wraps it, so there is no capability to name"
    );
}

/// The delegating case still resolves its capability, so narrowing the predicate
/// did not cost the structure it was narrowing for.
#[test]
fn a_delegated_operation_keeps_the_capability_that_wraps_it() {
    let outcome = parse();
    let record = outcome.project.cores[0].machines[0]
        .transitions
        .iter()
        .flat_map(|transition| &transition.effects)
        .find(|effect| effect.name == "AudioOperation::Record")
        .expect("the delegated operation is a request");
    assert_eq!(record.capability.as_deref(), Some("Audio"));
}

/// Removing a false positive is not a diagnostic: nothing about this fixture
/// defeated analysis, so nothing is reported.
#[test]
fn dropping_payload_paths_reports_nothing() {
    let outcome = parse();
    assert!(
        outcome.warnings.is_empty(),
        "expected a clean extraction, got: {:#?}",
        outcome.warnings
    );
}
