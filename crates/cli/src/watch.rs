//! `--watch`: rerun the generation whenever a `.rs` file under the source
//! directory changes (debounced).

use std::path::Path;
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::Duration;

use notify::{RecursiveMode, Watcher};

const DEBOUNCE: Duration = Duration::from_millis(300);

/// Runs `run` once, then again after every relevant filesystem change.
/// Only returns on watcher setup failure (Ctrl-C ends the process).
pub fn watch(src: &Path, run: impl Fn() -> ExitCode) -> ExitCode {
    run();

    let (sender, receiver) = mpsc::channel();
    let mut watcher = match notify::recommended_watcher(sender) {
        Ok(watcher) => watcher,
        Err(err) => {
            eprintln!("error: failed to create file watcher: {err}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(err) = watcher.watch(src, RecursiveMode::Recursive) {
        eprintln!("error: failed to watch {}: {err}", src.display());
        return ExitCode::FAILURE;
    }
    eprintln!("watching {} — Ctrl-C to stop", src.display());

    while let Ok(event) = receiver.recv() {
        if !is_relevant(&event) {
            continue;
        }
        // Debounce: absorb the burst of events an editor save produces.
        std::thread::sleep(DEBOUNCE);
        while receiver.try_recv().is_ok() {}

        eprintln!("change detected, regenerating…");
        run();
    }

    ExitCode::SUCCESS
}

fn is_relevant(event: &notify::Result<notify::Event>) -> bool {
    match event {
        Ok(event) => event
            .paths
            .iter()
            .any(|path| path.extension().is_some_and(|ext| ext == "rs")),
        Err(_) => false,
    }
}
