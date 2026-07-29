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
                Self::request_audio(super::AudioOperation::Start);
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
                model.recorder.session.state = RecorderState::Uploading;
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
