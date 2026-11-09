//! Watches the library folders and rescans when something in them changes,
//! so music added while Squirt is open shows up without a manual rescan.
//!
//! Changes come in bursts (a download writes a temp file, renames it, adds
//! art...), so a rescan starts once the folders have been quiet for a few
//! seconds. Scans are incremental, so an extra one is cheap.

use crate::Service;
use notify::{EventKind, RecursiveMode, Watcher};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// How long the folders must be quiet before rescanning.
const SETTLE: Duration = Duration::from_secs(3);

impl Service {
    /// Starts the folder watcher thread. Picks up added and removed library
    /// folders on its own.
    pub fn start_watcher(self: &Arc<Self>) {
        let svc = self.clone();
        std::thread::Builder::new()
            .name("squirt-watch".into())
            .spawn(move || svc.watch_loop())
            .expect("spawn watch thread");
    }

    fn watch_loop(self: Arc<Self>) {
        let (tx, rx) = mpsc::channel();
        let mut watcher = match notify::recommended_watcher(move |r: notify::Result<notify::Event>| {
            if let Ok(e) = r {
                let _ = tx.send(e);
            }
        }) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("folder watching unavailable: {e}");
                return;
            }
        };

        let mut watched: HashSet<PathBuf> = HashSet::new();
        let mut last_change: Option<Instant> = None;
        let mut last_sync = Instant::now() - Duration::from_secs(60);
        loop {
            // Follow the folder list (it changes when folders are added or removed).
            if last_sync.elapsed() >= Duration::from_secs(2) {
                last_sync = Instant::now();
                let want: HashSet<PathBuf> =
                    self.lib.lock().unwrap().folders().unwrap_or_default().into_iter().map(|f| f.path).collect();
                for p in watched.difference(&want) {
                    let _ = watcher.unwatch(p);
                }
                for p in want.difference(&watched) {
                    if let Err(e) = watcher.watch(p, RecursiveMode::Recursive) {
                        eprintln!("can't watch {}: {e}", p.display());
                    }
                }
                watched = want;
            }

            match rx.recv_timeout(Duration::from_millis(500)) {
                Ok(e) if counts(&e) => last_change = Some(Instant::now()),
                Ok(_) | Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
            if last_change.is_some_and(|t| t.elapsed() >= SETTLE) {
                last_change = None;
                self.rescan();
            }
        }
    }
}

/// Whether an event could change the library: files or folders created,
/// changed, renamed or removed, ignoring hidden files (.DS_Store, temp files).
fn counts(e: &notify::Event) -> bool {
    let kind = matches!(e.kind, EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) | EventKind::Any);
    kind && e.paths.iter().any(|p| !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')))
}
