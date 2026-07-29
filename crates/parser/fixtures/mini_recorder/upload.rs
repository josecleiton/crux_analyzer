/// Mirrors the finished take to the server.
///
/// Being folded into `RecorderState`, which already tracks the upload.
///
/// @deprecated
pub enum UploadState {
    Empty,
    Uploading,
    Synced,
}

pub struct UploadModel {
    pub upload: UploadState,
}

impl super::MiniRecorder {
    pub(super) fn update_upload(event: super::recorder::RecorderEvent, model: &mut super::Model) {
        match event {
            super::recorder::RecorderEvent::StopPressed
                if matches!(model.uploads.upload, UploadState::Empty) =>
            {
                model.uploads.upload = UploadState::Uploading;
            }
            super::recorder::RecorderEvent::UploadFinished
                if matches!(model.uploads.upload, UploadState::Uploading) =>
            {
                model.uploads.upload = UploadState::Synced;
            }
            _ => {}
        }
    }
}
