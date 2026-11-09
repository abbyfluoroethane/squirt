//! First launch: whether setup has run, the usual media folders to offer,
//! and installing the MTPZ key file the Zune needs.

use crate::{err, Result, Service};
use serde::Serialize;
use squirt_library::{count_media, Kind, Library};
use squirt_mtp::mtpz::MtpzKeys;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatus {
    /// Setup has been finished (or skipped) before, or the library already
    /// has folders from before setup existed.
    pub done: bool,
    /// The MTPZ key file is in place and readable.
    pub keys: bool,
    /// Where the key file goes, for display (`~/.mtpz-data`).
    pub keys_path: String,
    /// Why the key file could not be read, if it exists but is bad.
    pub keys_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestedFolder {
    pub path: String,
    pub kind: &'static str,
    /// Files of that kind found inside.
    pub count: usize,
    /// True if counting stopped early, so `count` is a lower bound.
    pub more: bool,
    /// Already in the library (this folder or one above it).
    pub watched: bool,
    /// A folder inside it is already in the library.
    pub partly: bool,
}

/// The system's standard media folders, per platform.
fn standard_folders() -> Vec<(PathBuf, Kind)> {
    let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).map(PathBuf::from) else {
        return Vec::new();
    };
    let videos = if cfg!(target_os = "macos") { "Movies" } else { "Videos" };
    [("Music", Kind::Music), (videos, Kind::Video), ("Pictures", Kind::Picture)]
        .into_iter()
        .map(|(d, k)| (home.join(d), k))
        .filter(|(p, _)| p.is_dir())
        .collect()
}

fn marker() -> Option<PathBuf> {
    Library::default_dirs().map(|(db, _)| db.with_file_name("setup-done"))
}

fn tilde(p: &Path) -> String {
    let s = p.to_string_lossy().into_owned();
    match std::env::var("HOME") {
        Ok(h) if !h.is_empty() && s.starts_with(&h) => format!("~{}", &s[h.len()..]),
        _ => s,
    }
}

impl Service {
    pub fn setup_status(&self) -> SetupStatus {
        let finished = marker().is_some_and(|m| m.exists());
        let has_folders = self.lib.lock().unwrap().folders().map(|f| !f.is_empty()).unwrap_or(false);
        let path = MtpzKeys::default_path();
        let (keys, keys_error) = match path.as_deref() {
            Some(p) if p.exists() => match MtpzKeys::load(p) {
                Ok(_) => (true, None),
                Err(e) => (false, Some(e.to_string())),
            },
            _ => match MtpzKeys::builtin() {
                Some(Ok(_)) => (true, None),
                Some(Err(e)) => (false, Some(format!("built-in keys: {e}"))),
                None => (false, None),
            },
        };
        SetupStatus {
            done: finished || has_folders,
            keys,
            keys_path: path.as_deref().map(tilde).unwrap_or_else(|| "~/.mtpz-data".into()),
            keys_error,
        }
    }

    /// The standard media folders that exist, with how much is in each.
    /// Counting runs in parallel and gives up after about a second per folder.
    pub fn setup_suggestions(&self) -> Vec<SuggestedFolder> {
        let watched: Vec<(PathBuf, Kind)> =
            self.lib.lock().unwrap().folders().unwrap_or_default().into_iter().map(|f| (f.path, f.kind)).collect();
        let handles: Vec<_> = standard_folders()
            .into_iter()
            .map(|(path, kind)| {
                std::thread::spawn(move || {
                    let (count, more) = count_media(&path, kind, 100_000, Duration::from_millis(1200));
                    (path, kind, count, more)
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|h| h.join().ok())
            .map(|(path, kind, count, more)| {
                let canon = path.canonicalize().unwrap_or_else(|_| path.clone());
                SuggestedFolder {
                    watched: watched.iter().any(|(w, k)| *k == kind && canon.starts_with(w)),
                    partly: watched.iter().any(|(w, _)| w.starts_with(&canon) && *w != canon),
                    path: path.to_string_lossy().into_owned(),
                    kind: kind.as_str(),
                    count,
                    more,
                }
            })
            .collect()
    }

    /// Checks a key file and copies it to `~/.mtpz-data`, then retries the device.
    pub fn import_keys(&self, path: &str) -> Result<SetupStatus> {
        let src = PathBuf::from(path);
        MtpzKeys::load(&src).map_err(|e| format!("not a usable key file: {e}"))?;
        let dest = MtpzKeys::default_path().ok_or("no home folder")?;
        if src.canonicalize().ok() != dest.canonicalize().ok() {
            std::fs::copy(&src, &dest).map_err(err)?;
        }
        self.device.lock().unwrap().checked = None;
        Ok(self.setup_status())
    }

    /// Marks setup as done, so it does not show again.
    pub fn finish_setup(&self) -> Result<SetupStatus> {
        let m = marker().ok_or("no data directory")?;
        std::fs::write(&m, b"").map_err(err)?;
        Ok(self.setup_status())
    }
}
