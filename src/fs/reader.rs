use core::panic;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

use crate::app::event::{AppEvent, Event, FileSystemChangeKind};

/// Receives requests to read files from the file system monitor. Should run in a separate thread.
/// This thread will read the file and send the contents back to the main thread.
/// The main thread will then process the file and update the UI accordingly.
pub fn start(rx: Receiver<PathBuf>, tx: Sender<Event>) {
    while let Ok(path) = rx.recv() {
        match read_to_string(&path) {
            Ok(content) => {
                let app_event = Event::App(AppEvent::FileSystemChanged(FileSystemChangeKind::Update(path, content)));

                // TODO:: Log on error
                if let Err(err) = tx.send(app_event) {
                    panic!("Failed to send file system change event: {err}");
                };
            },
            // TODO:: Log on error
            Err(err) => panic!("Failed to read file: {err}"),
        }
    }

    // TODO:: Log on error
    panic!("File system monitor thread exited unexpectedly");
}
