//! Fixture for state machines the core never writes as a literal variant path.
//!
//! `JobStatus` is held per entity inside a collection the `Model` owns, and
//! every write to it is either an event payload or a clone of another entry's
//! field. Literal-assignment evidence alone therefore finds nothing here; the
//! machine is detected because the field is reachable from the `Model`
//! associated type and the crate dispatches on the enum.
//!
//! `ViewStatus` is the control: same variants, dispatched on, and assigned —
//! but only into a view struct the model never holds, so it must stay out.

use crux_core::App;

pub enum Event {
    /// Fresh list from disk; statuses that only this run knows are carried over.
    Loaded { entries: Vec<Entry> },
    /// The shell reports where transcription of one entry got to.
    StatusReported { id: String, status: JobStatus },
    Opened { id: String },
}

pub enum Effect {
    Render,
}

/// How far one entry has got. The shell owns starting the work, so the core
/// only ever stores what it is told.
#[derive(Clone, PartialEq)]
pub enum JobStatus {
    /// Nothing has started yet.
    Pending,
    /// The shell is working on this entry.
    Running,
    /// A result is on disk.
    Done,
    /// Deferred until the network is worth spending on. Its own answer rather
    /// than a failure: nothing is wrong and nothing was refused.
    Deferred,
    /// Attempted and failed, or not supported here.
    Unavailable,
}

/// The mirror the view layer hands out. Never held by the `Model`.
#[derive(Clone, PartialEq)]
pub enum ViewStatus {
    Pending,
    Running,
    Done,
    Deferred,
    Unavailable,
}

pub struct Entry {
    pub id: String,
    pub status: JobStatus,
}

pub struct EntryView {
    pub id: String,
    /// Deliberately *not* named `status`: source constraints are keyed by field
    /// name, and this fixture is about reachability, not that coarseness.
    pub display_status: ViewStatus,
}

#[derive(Default)]
pub struct Board {
    pub entries: Vec<Entry>,
}

#[derive(Default)]
pub struct Model {
    pub board: Board,
}

pub struct ValueFlowStatus;

impl ValueFlowStatus {
    fn update(&self, event: Event, model: &mut Model) {
        match event {
            Event::Loaded { mut entries } => {
                for entry in &mut entries {
                    if let Some(known) = model.board.entries.iter().find(|e| e.id == entry.id) {
                        // Only this run knows these two, and a reload must not
                        // turn them back into "starting soon". The guard pairs a
                        // condition on the entry being written with one on the
                        // entry being read from — same field name, different
                        // objects.
                        if entry.status == JobStatus::Pending
                            && matches!(known.status, JobStatus::Deferred | JobStatus::Unavailable)
                        {
                            entry.status = known.status.clone();
                        }
                    }
                }
                model.board.entries = entries;
            }
            Event::StatusReported { id, status } => {
                if let Some(entry) = model.board.entries.iter_mut().find(|e| e.id == id) {
                    // Target supplied by the shell — a wildcard target.
                    entry.status = status;
                }
            }
            Event::Opened { id } => {
                let _ = self.view(model, &id);
            }
        }
    }

    /// The mirror enum's only life: *constructed* into a view struct, and
    /// dispatched on to pick the copy. Never assigned into anything the model
    /// holds, which is what keeps it out of the machine list.
    fn view(&self, model: &Model, id: &str) -> Option<EntryView> {
        let entry = model.board.entries.iter().find(|e| e.id == id)?;
        let display_status = match entry.status {
            JobStatus::Pending => ViewStatus::Pending,
            JobStatus::Running => ViewStatus::Running,
            JobStatus::Done => ViewStatus::Done,
            JobStatus::Deferred => ViewStatus::Deferred,
            JobStatus::Unavailable => ViewStatus::Unavailable,
        };
        Some(EntryView {
            id: entry.id.clone(),
            display_status,
        })
    }
}

impl App for ValueFlowStatus {
    type Event = Event;
    type Model = Model;
    type Effect = Effect;
}
