//! `--watch`: rerun the generation whenever a `.rs` file under the source
//! directory changes (debounced).

use std::path::Path;
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};

use crate::Messages;

const DEBOUNCE: Duration = Duration::from_millis(300);

/// Runs `run` once, then again after every relevant filesystem change.
/// Only returns on watcher setup failure (Ctrl-C ends the process).
///
/// `out` is the output path, if any: writing it must not be mistaken for a
/// source change, or `--out` inside `--src` with a `.rs` name would regenerate
/// forever.
pub fn watch(
    src: &Path,
    out: Option<&Path>,
    messages: &Messages,
    run: impl Fn() -> ExitCode,
) -> ExitCode {
    run();

    // Canonicalized once: the events carry absolute paths, and the output file
    // now exists (`run` just wrote it).
    let out = out.and_then(|path| path.canonicalize().ok());

    let (sender, receiver) = mpsc::channel();
    let mut watcher = match notify::recommended_watcher(sender) {
        Ok(watcher) => watcher,
        Err(err) => {
            eprintln!(
                "{}: {}: {err}",
                messages.error_prefix(),
                messages.failed_to_create_watcher()
            );
            return ExitCode::FAILURE;
        }
    };
    if let Err(err) = watcher.watch(src, RecursiveMode::Recursive) {
        eprintln!(
            "{}: {}: {err}",
            messages.error_prefix(),
            messages.failed_to_watch(src)
        );
        return ExitCode::FAILURE;
    }
    eprintln!("{}", messages.watching(src));

    while let Ok(event) = receiver.recv() {
        // A watcher error is reported rather than dropped: an inotify queue
        // overflow otherwise leaves the output silently stale.
        if let Err(err) = &event {
            eprintln!("{}: {err}", messages.warning_prefix());
            continue;
        }
        if !is_relevant(&event, out.as_deref()) {
            continue;
        }
        // Debounce: absorb the burst of events an editor save produces.
        std::thread::sleep(DEBOUNCE);
        while receiver.try_recv().is_ok() {}

        eprintln!("{}", messages.change_detected());
        run();
    }

    ExitCode::SUCCESS
}

fn is_relevant(event: &notify::Result<notify::Event>, out: Option<&Path>) -> bool {
    match event {
        Ok(event) => event.paths.iter().any(|path| {
            // A path that no longer resolves was deleted — still a change. Only
            // a path that resolves to the output file is ignored.
            path.extension().is_some_and(|ext| ext == "rs")
                && !matches!((path.canonicalize(), out), (Ok(p), Some(out)) if p == out)
        }),
        Err(_) => false,
    }
}
