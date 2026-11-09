//! Sending library tracks to the Zune in the background, with progress.
//!
//! `sync_tracks` adds tracks to a queue and returns at once; a worker thread
//! copies them one at a time, holding the device only while each track is
//! sent (so status checks don't wait for the whole sync). When the queue
//! empties it re-reads the device's music, so the collection's "on device"
//! marks update.

use crate::push::{read_meta, scale_art};
use crate::{err, Result, Service};
use serde::Serialize;
use squirt_mtp::library::PushOutcome;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    /// Copying now, or tracks still queued.
    pub active: bool,
    /// Tracks finished in this run (sent, already there, or failed).
    pub done: usize,
    pub total: usize,
    pub bytes_done: u64,
    pub bytes_total: u64,
    /// "artist — title" of the track being sent.
    pub current: Option<String>,
    /// Library ids still to do, the current one first.
    pub pending: Vec<i64>,
    /// "title: reason" for each track that failed in this run.
    pub errors: Vec<String>,
    /// Summary of the last finished run, e.g. "added 10 songs".
    pub last: Option<String>,
}

#[derive(Default)]
pub(crate) struct SyncState {
    status: SyncStatus,
    queue: VecDeque<i64>,
    current_id: Option<i64>,
    worker: bool,
    added: usize,
    skipped: usize,
}

impl SyncState {
    fn publish_pending(&mut self) {
        self.status.pending = self.current_id.into_iter().chain(self.queue.iter().copied()).collect();
    }
}

impl Service {
    /// Queues library tracks to send to the device. Tracks already queued
    /// are not added twice; tracks already on the device are skipped quickly.
    pub fn sync_tracks(self: &Arc<Self>, ids: Vec<i64>) -> Result<SyncStatus> {
        if self.device.lock().unwrap().zune.is_none() {
            return Err("no zune connected".into());
        }
        let sizes: HashMap<i64, u64> = {
            let lib = self.lib.lock().unwrap();
            ids.iter().filter_map(|&id| lib.track(id).ok().flatten().map(|t| (id, t.size))).collect()
        };
        let mut st = self.sync.lock().unwrap();
        if !st.worker {
            // A new run: start the counters over.
            let last = st.status.last.take();
            *st = SyncState::default();
            st.status.last = last;
        }
        for id in ids {
            if st.current_id == Some(id) || st.queue.contains(&id) {
                continue;
            }
            let Some(&size) = sizes.get(&id) else { continue };
            st.queue.push_back(id);
            st.status.total += 1;
            st.status.bytes_total += size;
        }
        st.status.active = st.current_id.is_some() || !st.queue.is_empty();
        st.publish_pending();
        if st.status.active && !st.worker {
            st.worker = true;
            let svc = self.clone();
            std::thread::Builder::new().name("squirt-sync".into()).spawn(move || svc.sync_loop()).map_err(err)?;
        }
        Ok(st.status.clone())
    }

    pub fn sync_status(&self) -> SyncStatus {
        self.sync.lock().unwrap().status.clone()
    }

    fn sync_loop(self: Arc<Self>) {
        // Scaled art per album, so each album's art is decoded once.
        let mut art: HashMap<(String, String), Option<Vec<u8>>> = HashMap::new();
        loop {
            let id = {
                let mut st = self.sync.lock().unwrap();
                st.current_id = st.queue.pop_front();
                st.publish_pending();
                match st.current_id {
                    Some(id) => id,
                    None => break,
                }
            };
            let (title, outcome) = self.sync_one(id, &mut art);
            let mut st = self.sync.lock().unwrap();
            st.status.done += 1;
            st.status.current = None;
            match outcome {
                Ok(true) => st.added += 1,
                Ok(false) => st.skipped += 1,
                Err(e) => {
                    if st.status.errors.len() < 50 {
                        st.status.errors.push(format!("{title}: {e}"));
                    }
                    if e == "the zune was disconnected" {
                        // Nothing else can go through; drop the rest.
                        let left = st.queue.len();
                        st.queue.clear();
                        st.status.done += left;
                    }
                }
            }
        }

        // Re-read the device's music so "on device" marks and free space update.
        {
            let mut dev = self.device.lock().unwrap();
            let storage = dev.storage;
            if let Some(z) = dev.zune.as_mut() {
                if let (Ok(tracks), Ok(albums)) = (z.music_tracks(), z.album_objects(storage)) {
                    self.set_device_library(&mut dev, Some(crate::DeviceLibrary { tracks, albums }));
                }
                dev.art.clear();
                dev.checked = None;
            }
        }

        let mut st = self.sync.lock().unwrap();
        let failed = st.status.errors.len();
        let mut parts = vec![format!("added {} song{}", st.added, if st.added == 1 { "" } else { "s" })];
        if st.skipped > 0 {
            parts.push(format!("{} already there", st.skipped));
        }
        if failed > 0 {
            parts.push(format!("{failed} failed"));
        }
        st.status.last = Some(parts.join(" · "));
        st.status.active = false;
        st.status.current = None;
        st.status.bytes_done = st.status.bytes_total;
        st.current_id = None;
        st.worker = false;
        st.publish_pending();
    }

    /// Sends one track. Ok(true) if sent, Ok(false) if it was already there.
    /// Also returns the title, for messages.
    fn sync_one(&self, id: i64, art: &mut HashMap<(String, String), Option<Vec<u8>>>) -> (String, std::result::Result<bool, String>) {
        let Some(t) = self.lib.lock().unwrap().track(id).ok().flatten() else {
            return (format!("track {id}"), Err("no longer in the library".into()));
        };
        let title = t.title.clone();
        let size = t.size;
        let base = {
            let mut st = self.sync.lock().unwrap();
            st.status.current = Some(format!("{} — {}", t.artist, t.title));
            st.status.bytes_done
        };
        let result = (|| {
            let meta = read_meta(&t.path)?;
            let key = (meta.track.album_artist.clone(), meta.track.album.clone());
            let cover = art.entry(key).or_insert_with(|| meta.art.as_deref().and_then(|a| scale_art(a).ok())).clone();
            let bytes = std::fs::read(&t.path).map_err(|e| e.to_string())?;
            let mut dev = self.device.lock().unwrap();
            let storage = dev.storage;
            let z = dev.zune.as_mut().ok_or("the zune was disconnected")?;
            let outcome = z
                .push_track_progress(storage, &meta.track, &bytes, cover.as_deref(), &mut |sent| {
                    self.sync.lock().unwrap().status.bytes_done = base + sent as u64;
                })
                .map_err(|e| e.to_string())?;
            Ok(matches!(outcome, PushOutcome::Pushed { .. }))
        })();
        // Count the whole file as done, sent or not, so the bar keeps moving.
        self.sync.lock().unwrap().status.bytes_done = base + size;
        (title, result)
    }
}
