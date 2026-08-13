//! Integration test over the `mini_recorder` fixture sources.

use std::path::Path;

use crux_analyzer_model::{Effect, Event, Marker, StateDecl};
use crux_analyzer_parser::parse_project;

#[test]
fn extracts_all_state_machines() {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/mini_recorder");
    let outcome = parse_project(&fixture, "Mini Recorder").expect("fixture must parse");

    assert_eq!(outcome.project.project, "Mini Recorder");
    assert_eq!(outcome.project.cores.len(), 1);

    let core = &outcome.project.cores[0];
    assert_eq!(core.name, "MiniRecorder");
    assert_eq!(core.machines.len(), 2, "two orthogonal regions expected");

    let recorder = core
        .machines
        .iter()
        .find(|m| m.name == "RecorderState")
        .expect("RecorderState machine");
    assert_eq!(
        recorder
            .states
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        [
            "Idle",
            "Recording",
            "Paused",
            "Uploading",
            "Completed",
            "Failed"
        ]
    );

    let triples: Vec<(&str, &str, &str)> = recorder
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
        // guard on a struct-variant state, one target per branch
        ("Failed", "RetryPressed", "Uploading"),
        ("Failed", "RetryPressed", "Idle"),
        // match-on-state helper with wildcard complement
        ("Recording", "Failed", "Failed"),
        ("Paused", "Failed", "Failed"),
        ("Uploading", "Failed", "Failed"),
    ];
    for triple in &expected {
        assert!(
            triples.contains(triple),
            "missing transition {triple:?} in {triples:#?}"
        );
    }
    assert_eq!(
        triples.len(),
        expected.len(),
        "unexpected extras: {triples:#?}"
    );

    // Second region: the upload machine, driven by the same events.
    let upload = core
        .machines
        .iter()
        .find(|m| m.name == "UploadState")
        .expect("UploadState machine");
    let upload_triples: Vec<(&str, &str, &str)> = upload
        .transitions
        .iter()
        .map(|t| (t.from.0.as_str(), t.event.0.as_str(), t.to.0.as_str()))
        .collect();
    assert_eq!(
        upload_triples,
        [
            ("Empty", "StopPressed", "Uploading"),
            ("Uploading", "UploadFinished", "Synced"),
        ]
    );

    // Effects requested by the arms reach their transitions.
    let effects_of = |event: &str, to: &str| {
        recorder
            .transitions
            .iter()
            .find(|t| t.event.0 == event && t.to.0 == to)
            .map(|t| t.effects.clone())
            .unwrap_or_default()
    };
    fn names(effects: &[Effect]) -> Vec<&str> {
        effects.iter().map(|e| e.name.as_str()).collect()
    }

    // A capability-style request: the operation, the capability its enum sits
    // under in `Effect`, and the event handed to the same call.
    assert_eq!(
        effects_of("RecordPressed", "Recording"),
        [Effect {
            name: "AudioOperation::Start".into(),
            capability: Some("Audio".into()),
            resolves_with: vec![Event("CaptureStarted".into())],
            conditional: false,
        }]
    );

    // Stopping always tells the hardware; sending the take sits on a branch
    // below the assignment, so it is kept and marked conditional.
    let stopping = effects_of("StopPressed", "Uploading");
    assert_eq!(
        names(&stopping),
        ["AudioOperation::Stop", "HttpOperation::Upload"]
    );
    assert!(!stopping[0].conditional);
    assert!(
        stopping[0].resolves_with.is_empty(),
        "fire-and-forget request"
    );
    assert!(stopping[1].conditional);
    assert_eq!(stopping[1].capability.as_deref(), Some("Http"));
    // The other half of the loop: the event the shell answers with is an event
    // this very machine handles.
    assert_eq!(stopping[1].resolves_with, [Event("UploadFinished".into())]);

    // Two branches of one arm, two targets, and neither inherits the other's
    // request — the retry uploads, giving up only renders.
    assert_eq!(
        names(&effects_of("RetryPressed", "Uploading")),
        ["HttpOperation::Upload"]
    );
    assert_eq!(names(&effects_of("RetryPressed", "Idle")), ["Render"]);
    // `render()` goes through no capability and answers with nothing.
    assert!(effects_of("RetryPressed", "Idle")[0].is_bare());

    // Doc comments on event and effect variants become the core's catalogs —
    // only the documented AND used names.
    let events: Vec<(&str, &str)> = core
        .events
        .iter()
        .map(|e| (e.name.as_str(), e.doc.as_str()))
        .collect();
    assert_eq!(
        events,
        [
            // Named only as an effect's callback, and documented — no
            // transition carries it, and it is still part of the model.
            (
                "CaptureStarted",
                "The shell confirmed the microphone is live. Nothing to decide: the\nsession is already recording."
            ),
            ("RecordPressed", "The user hit the record button on the main screen."),
            ("RetryPressed", "Retry the failed upload, keeping the recorded take."),
        ]
    );
    let effects: Vec<(&str, &str)> = core
        .effects
        .iter()
        .map(|e| (e.name.as_str(), e.doc.as_str()))
        .collect();
    assert_eq!(
        effects,
        [
            (
                "AudioOperation::Start",
                "Arms the microphone and begins capturing into the session buffer."
            ),
            (
                "HttpOperation::Upload",
                "Sends the finished take, answering with the server's verdict."
            ),
        ]
    );

    // Documentation authored in the fixture reaches the model.
    assert_eq!(
        recorder.doc.as_deref(),
        Some(
            "Where one recording session lives, from arming the microphone to a\nfinished upload."
        )
    );
    let state = |name: &str| {
        recorder
            .states
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("no state {name}"))
    };
    assert_eq!(
        state("Idle").doc.as_deref(),
        Some("Nothing is being recorded yet. Every session starts and ends here.")
    );
    // A declared marker and tag, with the annotation lines out of the prose.
    let failed = state("Failed");
    assert_eq!(
        failed.doc.as_deref(),
        Some("The upload gave up. The session is kept so it can be sent again.")
    );
    assert_eq!(failed.markers, [Marker::Failure]);
    assert_eq!(failed.tags, ["retryable"]);
    // A multi-paragraph description keeps its break.
    assert!(state("Paused").doc.as_deref().unwrap().contains("\n\n"));

    // The upload region is marked at the machine level and documents no state,
    // so it also covers the all-bare path in the same run.
    assert_eq!(upload.markers, [Marker::Deprecated]);
    assert!(upload.doc.as_deref().unwrap().starts_with("Mirrors"));
    assert!(
        upload.states.iter().all(StateDecl::is_bare),
        "{:#?}",
        upload.states
    );

    assert!(
        outcome.warnings.is_empty(),
        "unexpected warnings: {:#?}",
        outcome.warnings
    );
}
