//! Corpus test against a real Crux application (quipu_app_crux).
//!
//! Gated on the `QUIPU_SRC` env var pointing at the app's `src` directory:
//!
//! ```sh
//! QUIPU_SRC=~/dev/projects/personal/quipu_app_crux/shared/src cargo test -p crux-analyzer-parser
//! ```
//!
//! Skips (with a message) when the variable is unset, so CI without the
//! corpus stays green.

use std::path::PathBuf;

use crux_analyzer_parser::parse_project;

#[test]
fn extracts_quipu_recording_state_machine() {
    let Ok(src) = std::env::var("QUIPU_SRC") else {
        eprintln!("skipping: set QUIPU_SRC to the quipu_app_crux src directory to run");
        return;
    };
    let outcome = parse_project(&PathBuf::from(src), "Quipu").expect("quipu must parse");

    let core = outcome
        .project
        .cores
        .iter()
        .find(|c| c.name == "Quipu")
        .expect("core Quipu not found");

    let machine = core
        .machines
        .iter()
        .find(|m| m.name == "RecordingState")
        .expect("RecordingState machine not found");

    let states: Vec<&str> = machine.states.iter().map(|s| s.0.as_str()).collect();
    assert_eq!(
        states,
        [
            "Idle",
            "Starting",
            "Recording",
            "PausedByUser",
            "PausedByInterruption",
            "Resuming",
            "Stopping",
            "FinishedAwaitingDecision",
        ]
    );

    let triples: Vec<(&str, &str, &str)> = machine
        .transitions
        .iter()
        .map(|t| (t.from.0.as_str(), t.event.0.as_str(), t.to.0.as_str()))
        .collect();

    let expected = [
        // capture controls
        ("Idle", "StartTapped", "Starting"),
        ("Recording", "PauseTapped", "PausedByUser"),
        ("PausedByUser", "ResumeTapped", "Resuming"),
        ("PausedByInterruption", "ResumeTapped", "Resuming"),
        ("Recording", "StopTapped", "Stopping"),
        ("PausedByUser", "StopTapped", "Stopping"),
        ("PausedByInterruption", "StopTapped", "Stopping"),
        ("Resuming", "StopTapped", "Stopping"),
        ("FinishedAwaitingDecision", "DecisionDismissed", "PausedByUser"),
        // settle(): CaptureSession::default() reset → #[default] Idle
        ("FinishedAwaitingDecision", "FinalizeTapped", "Idle"),
        ("FinishedAwaitingDecision", "SaveDraftTapped", "Idle"),
        // settle() via confirmation flow: no state guard → wildcard source
        ("*", "ConfirmationAccepted", "Idle"),
        // capture reports
        ("Starting", "RecordingStarted", "Recording"),
        ("Resuming", "RecordingResumed", "Recording"),
        ("Stopping", "RecordingFinished", "FinishedAwaitingDecision"),
        // park_failed_capture: match-on-state with wildcard complement
        ("Starting", "RecordingFailed", "Idle"),
        ("Recording", "RecordingFailed", "PausedByUser"),
        ("PausedByUser", "RecordingFailed", "PausedByUser"),
        ("PausedByInterruption", "RecordingFailed", "PausedByUser"),
        ("Resuming", "RecordingFailed", "PausedByUser"),
        ("Stopping", "RecordingFailed", "PausedByUser"),
        // bury_dead_capture: guarded by the has_capture_in_flight() predicate
        ("Recording", "CaptureDied", "FinishedAwaitingDecision"),
        ("PausedByUser", "CaptureDied", "FinishedAwaitingDecision"),
        ("PausedByInterruption", "CaptureDied", "FinishedAwaitingDecision"),
        ("Resuming", "CaptureDied", "FinishedAwaitingDecision"),
        ("Stopping", "CaptureDied", "FinishedAwaitingDecision"),
        // recovery + interruptions
        ("Idle", "RecoveredRecordings", "FinishedAwaitingDecision"),
        ("Recording", "InterruptionBegan", "PausedByInterruption"),
        ("PausedByInterruption", "InterruptionEnded", "Resuming"),
        // drafts
        ("Idle", "DraftResumeRequested", "PausedByUser"),
    ];
    for triple in &expected {
        assert!(triples.contains(triple), "missing transition {triple:?} in {triples:#?}");
    }
    assert_eq!(triples.len(), expected.len(), "unexpected extras: {triples:#?}");

    // A second region exists in the corpus: the insight pipeline status.
    assert!(
        core.machines.iter().any(|m| m.name == "InsightStatus"),
        "expected the InsightStatus machine, got: {:?}",
        core.machines.iter().map(|m| &m.name).collect::<Vec<_>>()
    );

    // Everything in the capture flow is now statically resolvable.
    assert!(
        outcome.warnings.is_empty(),
        "expected no warnings, got: {:#?}",
        outcome.warnings
    );
}
