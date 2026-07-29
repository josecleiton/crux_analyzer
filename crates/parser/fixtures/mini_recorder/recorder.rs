pub enum RecorderEvent {
    /// The user hit the record button on the main screen.
    RecordPressed,
    PausePressed,
    ResumePressed,
    StopPressed,
    UploadFinished,
    /// Retry the failed upload, keeping the recorded take.
    RetryPressed,
    Failed,
    /// The shell confirmed the microphone is live. Nothing to decide: the
    /// session is already recording.
    CaptureStarted,
}

/// Where one recording session lives, from arming the microphone to a
/// finished upload.
pub enum RecorderState {
    /// Nothing is being recorded yet. Every session starts and ends here.
    Idle,
    /// Capturing audio from the microphone.
    Recording,
    /// Capture is suspended and the buffer is kept.
    ///
    /// `by_system` distinguishes a pause the user asked for from one an
    /// interruption forced.
    Paused { by_system: bool },
    /// The finished take is on its way to the server.
    Uploading,
    /// The take is stored and the session is done.
    Completed,
    /// The upload gave up. The session is kept so it can be sent again.
    ///
    /// @failure
    /// @tag retryable
    Failed { reason: String },
}

pub struct Session {
    pub state: RecorderState,
}

pub struct RecorderModel {
    pub session: Session,
}

impl super::MiniRecorder {
    pub(super) fn update_recorder(event: RecorderEvent, model: &mut super::Model) {
        match event {
            RecorderEvent::RecordPressed
                if matches!(model.recorder.session.state, RecorderState::Idle) =>
            {
                model.recorder.session.state = RecorderState::Recording;
                // Capability-style request: the event the shell answers with
                // travels alongside the operation.
                Self::request_audio_then(
                    super::AudioOperation::Start,
                    RecorderEvent::CaptureStarted,
                );
            }
            RecorderEvent::PausePressed
                if matches!(model.recorder.session.state, RecorderState::Recording) =>
            {
                model.recorder.session.state = RecorderState::Paused { by_system: false };
            }
            RecorderEvent::ResumePressed
                if matches!(model.recorder.session.state, RecorderState::Paused { .. }) =>
            {
                model.recorder.session.state = RecorderState::Recording;
            }
            event @ (RecorderEvent::StopPressed | RecorderEvent::UploadFinished) => {
                Self::finish(event, model)
            }
            RecorderEvent::RetryPressed
                if matches!(model.recorder.session.state, RecorderState::Failed { .. }) =>
            {
                // Two branches, two different requests: neither belongs to the
                // other's transition.
                if model.recorder.session.attempts_left() {
                    model.recorder.session.state = RecorderState::Uploading;
                    Self::request(super::HttpOperation::Upload)
                        .then_send(RecorderEvent::UploadFinished);
                } else {
                    model.recorder.session.state = RecorderState::Idle;
                    render();
                }
            }
            RecorderEvent::Failed => Self::park(&mut model.recorder.session),
            _ => {}
        }
    }

    fn finish(event: RecorderEvent, model: &mut super::Model) {
        match event {
            RecorderEvent::StopPressed
                if matches!(
                    model.recorder.session.state,
                    RecorderState::Recording | RecorderState::Paused { .. }
                ) =>
            {
                model.recorder.session.state = RecorderState::Uploading;
                Self::request_audio(super::AudioOperation::Stop);
                // Requested on a branch below the assignment: arriving in
                // `Uploading` *may* send the take, it does not always.
                if model.recorder.session.is_last_take() {
                    Self::request(super::HttpOperation::Upload)
                        .then_send(RecorderEvent::UploadFinished);
                }
            }
            RecorderEvent::UploadFinished
                if matches!(model.recorder.session.state, RecorderState::Uploading) =>
            {
                model.recorder.session.state = RecorderState::Completed;
            }
            _ => {}
        }
    }

    fn park(session: &mut Session) {
        match session.state {
            RecorderState::Idle | RecorderState::Completed | RecorderState::Failed { .. } => {}
            _ => {
                session.state = RecorderState::Failed {
                    reason: String::new(),
                };
            }
        }
    }
}
