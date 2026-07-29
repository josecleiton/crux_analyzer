//! Minimal Crux-shaped app replicating the patterns found in real apps
//! (nested event enums, helper delegation, matches! guards, match-on-state).
//! Not a compiled crate — parsed as plain sources by the integration test.

mod recorder;
mod upload;

pub enum Event {
    Recorder(recorder::RecorderEvent),
}

/// Operations the shell performs against the audio hardware.
pub enum AudioOperation {
    /// Arms the microphone and begins capturing into the session buffer.
    Start,
    Stop,
}

pub enum Effect {
    Audio(AudioOperation),
}

pub struct Model {
    recorder: recorder::RecorderModel,
    uploads: upload::UploadModel,
}

pub struct MiniRecorder;

impl App for MiniRecorder {
    type Event = Event;
    type Effect = Effect;
    type Model = Model;

    fn update(&self, event: Event, model: &mut Model) {
        match event {
            Event::Recorder(event) => {
                Self::update_upload(event.clone(), model);
                Self::update_recorder(event, model)
            }
        }
    }
}
