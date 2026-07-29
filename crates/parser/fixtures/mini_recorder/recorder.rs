pub enum RecorderEvent {
    RecordPressed,
    PausePressed,
    ResumePressed,
    StopPressed,
    UploadFinished,
    Failed,
}

pub enum RecorderState {
    Idle,
    Recording,
    Paused { by_system: bool },
    Uploading,
    Completed,
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
            RecorderState::Idle | RecorderState::Completed => {}
            _ => {
                session.state = RecorderState::Idle;
            }
        }
    }
}
