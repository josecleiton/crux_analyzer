//! Minimal Crux-shaped app replicating the patterns found in real apps
//! (nested event enums, helper delegation, matches! guards, match-on-state).
//! Not a compiled crate — parsed as plain sources by the integration test.

mod recorder;

pub enum Event {
    Recorder(recorder::RecorderEvent),
}

pub struct Model {
    recorder: recorder::RecorderModel,
}

pub struct MiniRecorder;

impl App for MiniRecorder {
    type Event = Event;
    type Model = Model;

    fn update(&self, event: Event, model: &mut Model) {
        match event {
            Event::Recorder(event) => Self::update_recorder(event, model),
        }
    }
}
