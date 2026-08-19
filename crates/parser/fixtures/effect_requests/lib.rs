//! Fixture for the line between an effect *request* and effect *payload*.
//!
//! The effect closure walks variant field types transitively, which is what lets
//! `Effect::Audio(AudioOperation)` be found at all. Walked without a bound it
//! keeps going: `AudioOperation::Record { outcome: Outcome }` pulls in `Outcome`,
//! and `Outcome { domain: Domain }` pulls in `Domain` — after which every mention
//! of `Domain::Capture` anywhere in an update body reads as a request the shell is
//! asked to perform. Only what the root wraps *directly* is a request; everything
//! deeper is data travelling inside one.
//!
//! Two shapes here are not requests and must not be recorded:
//!
//! - a variant of a payload enum reached at depth 2 or deeper (`Domain::Capture`,
//!   `Outcome::Refused`);
//! - an associated function on an enum that *is* an operation
//!   (`AudioOperation::of`) — a call, not a variant, and the only way to tell is
//!   to ask whether the enum declares that name.
//!
//! Two are, and both are easy to erase by over-correcting:
//!
//! - `Effect::Announce`, an operation the root carries as its own variant rather
//!   than delegating to an operation enum. `capability_of` answers `None` for the
//!   root exactly as it does for a payload enum, so a predicate asking only
//!   "does this have a capability?" would drop it;
//!   ({`crux_analyzer` `docs/roadmap.md` §8, P3a).
//! - a bare `render()`, which arrives by another path entirely and is here as the
//!   control that says so.

use crux_core::App;

pub enum Event {
    Started,
    Refused,
    Announced,
}

/// Requests against the audio hardware.
pub enum AudioOperation {
    /// Arm the microphone and capture into the session buffer.
    Record { outcome: Outcome },
    Stop,
}

impl AudioOperation {
    /// Which request a raw outcome belongs to. A classifier, not a request — and
    /// spelled like one, which is the whole difficulty.
    pub fn of(outcome: &Outcome) -> Self {
        match outcome {
            Outcome::Refused { .. } => Self::Stop,
            Outcome::Landed => Self::Record {
                outcome: Outcome::Landed,
            },
        }
    }
}

/// What came back, as data. Reached only as a field of a request, so nothing here
/// is something the shell can be asked to do.
pub enum Outcome {
    /// The bytes arrived.
    Landed,
    /// The device said no, and would say no again.
    Refused { domain: Domain },
}

/// Which part of the app a failure belongs to. Depth 3, and the case that made
/// the loudest noise in a real core: a classifier mentioned on nearly every
/// branch, reported as an effect on every one of them.
pub enum Domain {
    Capture,
    Playback,
}

pub enum Effect {
    Audio(AudioOperation),
    /// An operation the root carries itself. Nothing wraps it, so it has no
    /// capability — and it is still a request.
    Announce,
}

/// Where the take stands.
#[derive(Default, PartialEq)]
pub enum Status {
    /// Nothing captured yet.
    #[default]
    Idle,
    /// Capturing.
    Recording,
    /// The device refused.
    Refused,
    /// Said out loud.
    Announced,
}

pub struct Model {
    pub status: Status,
}

pub struct Probe;

impl App for Probe {
    type Event = Event;
    type Effect = Effect;
    type Model = Model;

    fn update(&self, event: Event, model: &mut Model) {
        match event {
            // One request, and two payload variants built to fill it.
            Event::Started => {
                model.status = Status::Recording;
                Self::request(AudioOperation::Record {
                    outcome: Outcome::Landed,
                });
            }
            // A classifier call and a depth-3 payload variant. Neither is a
            // request; the `Stop` the classifier returns is not one either,
            // because nothing here asks for it.
            Event::Refused => {
                model.status = Status::Refused;
                let outcome = Outcome::Refused {
                    domain: Domain::Capture,
                };
                let _ = AudioOperation::of(&outcome);
                render();
            }
            // The root-carried operation, plus the bare render that arrives by
            // its own path.
            Event::Announced => {
                model.status = Status::Announced;
                Self::announce(Effect::Announce);
                render();
            }
        }
    }
}

impl Probe {
    fn request(operation: AudioOperation) {
        Command::request_from_shell(operation);
    }

    fn announce(effect: Effect) {
        Command::request_from_shell(effect);
    }
}
