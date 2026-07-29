//! Integration test over the `mini_recorder` fixture sources.

use std::path::Path;

use crux_analyzer_parser::parse_project;

#[test]
fn extracts_the_full_state_machine() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/mini_recorder");
    let outcome = parse_project(&fixture, "Mini Recorder").expect("fixture must parse");

    assert_eq!(outcome.project.project, "Mini Recorder");
    assert_eq!(outcome.project.cores.len(), 1);

    let core = &outcome.project.cores[0];
    assert_eq!(core.name, "MiniRecorder");
    assert_eq!(
        core.states.iter().map(|s| s.0.as_str()).collect::<Vec<_>>(),
        ["Idle", "Recording", "Paused", "Uploading", "Completed"]
    );

    let triples: Vec<(&str, &str, &str)> = core
        .transitions
        .iter()
        .map(|t| (t.from.0.as_str(), t.event.0.as_str(), t.to.0.as_str()))
        .collect();

    let expected = [
        // direct guard + assignment
        ("Idle", "RecordPressed", "Recording"),
        ("Recording", "PausePressed", "Paused"),
        ("Paused", "ResumePressed", "Recording"),
        // multi-event arm delegating to a helper, multi-pattern guard fan-out
        ("Recording", "StopPressed", "Uploading"),
        ("Paused", "StopPressed", "Uploading"),
        ("Uploading", "UploadFinished", "Completed"),
        // match-on-state helper with wildcard complement
        ("Recording", "Failed", "Idle"),
        ("Paused", "Failed", "Idle"),
        ("Uploading", "Failed", "Idle"),
    ];
    for triple in &expected {
        assert!(triples.contains(triple), "missing transition {triple:?} in {triples:#?}");
    }
    assert_eq!(triples.len(), expected.len(), "unexpected extras: {triples:#?}");
    assert!(outcome.warnings.is_empty(), "unexpected warnings: {:#?}", outcome.warnings);
}
