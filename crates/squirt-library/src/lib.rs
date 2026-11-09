//! Squirt's music library: a SQLite index of the folders the user watches.
//!
//! Files are never moved or changed by scanning. A scan reads tags, audio
//! properties and cover art; only files whose size or modification time
//! changed since the last scan are read again. Cover art is stored once per
//! distinct image in a cache directory, as a JPEG thumbnail.

mod meta;

pub use meta::{read_track, Codec, TrackMeta};

use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

/// Extensions the scanner looks at.
pub const AUDIO_EXTENSIONS: &[&str] = &["mp3", "m4a", "mp4", "aac", "flac", "ogg", "opus", "wma", "wav", "aif", "aiff"];

/// Longest edge of cached art thumbnails, in pixels. Twice the 240 px the
/// Zune uses, so it stays sharp on high-DPI screens.
pub const ART_SIZE: u32 = 480;

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0} is not a folder")]
    NotAFolder(PathBuf),
}

pub type Result<T> = std::result::Result<T, LibraryError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub id: i64,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct Track {
    pub id: i64,
    pub path: PathBuf,
    pub size: u64,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_no: Option<u32>,
    pub disc_no: Option<u32>,
    pub disc_total: Option<u32>,
    pub duration_ms: u64,
    pub codec: String,
    pub bitrate_kbps: Option<u32>,
    /// Cache key of the cover art, if any. See [`Library::art_path`].
    pub art: Option<String>,
    pub rating: u8,
    pub play_count: u32,
}

#[derive(Debug, Clone)]
pub struct Album {
    pub title: String,
    pub artist: String,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub track_count: u32,
    pub duration_ms: u64,
    pub size: u64,
    pub art: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct ScanReport {
    pub seen: usize,
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    pub errors: Vec<(PathBuf, String)>,
}

/// Progress for a running scan.
#[derive(Debug, Clone)]
pub struct ScanProgress<'a> {
    pub done: usize,
    pub total: usize,
    pub path: &'a Path,
}

pub struct Library {
    db: Connection,
    art_dir: PathBuf,
}

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS folders (
    id   INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE
);
CREATE TABLE IF NOT EXISTS tracks (
    id           INTEGER PRIMARY KEY,
    path         TEXT NOT NULL UNIQUE,
    folder_id    INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    size         INTEGER NOT NULL,
    mtime        INTEGER NOT NULL,
    title        TEXT NOT NULL,
    artist       TEXT NOT NULL,
    album        TEXT NOT NULL,
    album_artist TEXT NOT NULL,
    genre        TEXT,
    year         INTEGER,
    track_no     INTEGER,
    track_total  INTEGER,
    disc_no      INTEGER,
    disc_total   INTEGER,
    duration_ms  INTEGER NOT NULL,
    codec        TEXT NOT NULL,
    bitrate_kbps INTEGER,
    art          TEXT,
    rating       INTEGER NOT NULL DEFAULT 0,
    play_count   INTEGER NOT NULL DEFAULT 0,
    last_played  INTEGER,
    added_at     INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS tracks_album ON tracks(album_artist, album);
CREATE INDEX IF NOT EXISTS tracks_artist ON tracks(artist);
"#;

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn mtime_of(md: &std::fs::Metadata) -> i64 {
    md.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn is_audio(p: &Path) -> bool {
    p.extension()
        .map(|e| AUDIO_EXTENSIONS.contains(&e.to_string_lossy().to_lowercase().as_str()))
        .unwrap_or(false)
}

impl Library {
    /// Opens (creating if needed) a library whose database lives at `db_path`
    /// and whose art cache lives in `art_dir`.
    pub fn open(db_path: &Path, art_dir: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::create_dir_all(art_dir)?;
        let db = Connection::open(db_path)?;
        db.execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")?;
        db.execute_batch(SCHEMA)?;
        Ok(Self { db, art_dir: art_dir.to_path_buf() })
    }

    /// The default location: `<data dir>/squirt/library.db` and `.../art/`.
    pub fn default_dirs() -> Option<(PathBuf, PathBuf)> {
        let base = if cfg!(target_os = "macos") {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
        } else if cfg!(windows) {
            std::env::var_os("APPDATA").map(PathBuf::from)
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        }?;
        let dir = base.join("squirt");
        Some((dir.join("library.db"), dir.join("art")))
    }

    // ------------------------------------------------------------ folders

    pub fn add_folder(&self, path: &Path) -> Result<Folder> {
        let path = path.canonicalize()?;
        if !path.is_dir() {
            return Err(LibraryError::NotAFolder(path));
        }
        let s = path.to_string_lossy();
        self.db.execute("INSERT OR IGNORE INTO folders(path) VALUES (?1)", [&*s])?;
        let id = self.db.query_row("SELECT id FROM folders WHERE path = ?1", [&*s], |r| r.get(0))?;
        Ok(Folder { id, path })
    }

    /// Stops watching a folder and forgets its tracks (files are untouched).
    pub fn remove_folder(&self, id: i64) -> Result<()> {
        self.db.execute("DELETE FROM folders WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn folders(&self) -> Result<Vec<Folder>> {
        let mut st = self.db.prepare("SELECT id, path FROM folders ORDER BY path")?;
        let rows = st.query_map([], |r| Ok(Folder { id: r.get(0)?, path: PathBuf::from(r.get::<_, String>(1)?) }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // --------------------------------------------------------------- scan

    /// Scans every watched folder: adds new files, re-reads changed ones and
    /// forgets files that are gone.
    pub fn scan(&mut self, progress: &mut dyn FnMut(ScanProgress)) -> Result<ScanReport> {
        let mut report = ScanReport::default();
        let folders = self.folders()?;

        let mut files: Vec<(i64, PathBuf)> = Vec::new();
        for f in &folders {
            for entry in walkdir::WalkDir::new(&f.path).follow_links(true).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                let hidden = entry.file_name().to_string_lossy().starts_with('.');
                if entry.file_type().is_file() && !hidden && is_audio(p) {
                    files.push((f.id, p.to_path_buf()));
                }
            }
        }
        report.seen = files.len();

        let mut present: HashSet<String> = HashSet::new();
        let total = files.len();
        let tx = self.db.transaction()?;
        for (i, (folder_id, path)) in files.iter().enumerate() {
            progress(ScanProgress { done: i, total, path });
            let key = path.to_string_lossy().into_owned();
            present.insert(key.clone());
            let md = match std::fs::metadata(path) {
                Ok(m) => m,
                Err(e) => {
                    report.errors.push((path.clone(), e.to_string()));
                    continue;
                }
            };
            let (size, mtime) = (md.len() as i64, mtime_of(&md));
            let known: Option<(i64, i64)> = tx
                .query_row("SELECT size, mtime FROM tracks WHERE path = ?1", [&key], |r| Ok((r.get(0)?, r.get(1)?)))
                .optional()?;
            if known == Some((size, mtime)) {
                continue;
            }
            let meta = match read_track(path) {
                Ok(m) => m,
                Err(e) => {
                    report.errors.push((path.clone(), e));
                    continue;
                }
            };
            let art = meta.art.as_deref().and_then(|a| cache_art(&self.art_dir, a).ok());
            tx.execute(
                "INSERT INTO tracks (path, folder_id, size, mtime, title, artist, album, album_artist, genre, year,
                    track_no, track_total, disc_no, disc_total, duration_ms, codec, bitrate_kbps, art, added_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
                 ON CONFLICT(path) DO UPDATE SET
                    folder_id = excluded.folder_id, size = excluded.size, mtime = excluded.mtime,
                    title = excluded.title, artist = excluded.artist, album = excluded.album,
                    album_artist = excluded.album_artist, genre = excluded.genre, year = excluded.year,
                    track_no = excluded.track_no, track_total = excluded.track_total,
                    disc_no = excluded.disc_no, disc_total = excluded.disc_total,
                    duration_ms = excluded.duration_ms, codec = excluded.codec,
                    bitrate_kbps = excluded.bitrate_kbps, art = excluded.art",
                params![
                    key, folder_id, size, mtime, meta.title, meta.artist, meta.album, meta.album_artist, meta.genre,
                    meta.year, meta.track_no, meta.track_total, meta.disc_no, meta.disc_total,
                    meta.duration_ms as i64, meta.codec.as_str(), meta.bitrate_kbps, art, now()
                ],
            )?;
            if known.is_some() {
                report.updated += 1;
            } else {
                report.added += 1;
            }
        }

        // Forget tracks whose files are gone.
        let gone: Vec<(i64, String)> = {
            let mut st = tx.prepare("SELECT id, path FROM tracks")?;
            let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<rusqlite::Result<Vec<_>>>()?
                .into_iter()
                .filter(|(_, p): &(i64, String)| !present.contains(p))
                .collect()
        };
        for (id, _) in &gone {
            tx.execute("DELETE FROM tracks WHERE id = ?1", [id])?;
        }
        report.removed = gone.len();
        tx.commit()?;
        Ok(report)
    }

    // ------------------------------------------------------------ queries

    /// Path of a cached art thumbnail.
    pub fn art_path(&self, key: &str) -> PathBuf {
        self.art_dir.join(format!("{key}.jpg"))
    }

    pub fn artists(&self) -> Result<Vec<String>> {
        let mut st = self
            .db
            .prepare("SELECT DISTINCT album_artist FROM tracks ORDER BY album_artist COLLATE NOCASE")?;
        let rows = st.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Albums, optionally for one album artist, sorted by artist then year.
    pub fn albums(&self, artist: Option<&str>) -> Result<Vec<Album>> {
        let mut st = self.db.prepare(
            "SELECT album, album_artist, MAX(year), MAX(genre), COUNT(*), SUM(duration_ms), SUM(size),
                    (SELECT art FROM tracks t2 WHERE t2.album = t.album AND t2.album_artist = t.album_artist
                       AND art IS NOT NULL ORDER BY disc_no, track_no LIMIT 1)
             FROM tracks t
             WHERE ?1 IS NULL OR album_artist = ?1
             GROUP BY album_artist, album
             ORDER BY album_artist COLLATE NOCASE, MAX(year), album COLLATE NOCASE",
        )?;
        let rows = st.query_map([artist], |r| {
            Ok(Album {
                title: r.get(0)?,
                artist: r.get(1)?,
                year: r.get(2)?,
                genre: r.get(3)?,
                track_count: r.get(4)?,
                duration_ms: r.get::<_, i64>(5)? as u64,
                size: r.get::<_, i64>(6)? as u64,
                art: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Tracks of one album, in disc and track order.
    pub fn album_tracks(&self, artist: &str, album: &str) -> Result<Vec<Track>> {
        self.tracks_where("album_artist = ?1 AND album = ?2", params![artist, album])
    }

    pub fn track(&self, id: i64) -> Result<Option<Track>> {
        Ok(self.tracks_where("id = ?1", params![id])?.into_iter().next())
    }

    fn tracks_where(&self, cond: &str, p: impl rusqlite::Params) -> Result<Vec<Track>> {
        let sql = format!(
            "SELECT id, path, size, title, artist, album, album_artist, genre, year, track_no, disc_no, disc_total,
                    duration_ms, codec, bitrate_kbps, art, rating, play_count
             FROM tracks WHERE {cond}
             ORDER BY album_artist COLLATE NOCASE, album COLLATE NOCASE, COALESCE(disc_no, 1), track_no, title"
        );
        let mut st = self.db.prepare(&sql)?;
        let rows = st.query_map(p, |r| {
            Ok(Track {
                id: r.get(0)?,
                path: PathBuf::from(r.get::<_, String>(1)?),
                size: r.get::<_, i64>(2)? as u64,
                title: r.get(3)?,
                artist: r.get(4)?,
                album: r.get(5)?,
                album_artist: r.get(6)?,
                genre: r.get(7)?,
                year: r.get(8)?,
                track_no: r.get(9)?,
                disc_no: r.get(10)?,
                disc_total: r.get(11)?,
                duration_ms: r.get::<_, i64>(12)? as u64,
                codec: r.get(13)?,
                bitrate_kbps: r.get(14)?,
                art: r.get(15)?,
                rating: r.get(16)?,
                play_count: r.get(17)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Counts a play: called by the player when a track finishes (or mostly).
    pub fn record_play(&self, id: i64) -> Result<()> {
        self.db.execute(
            "UPDATE tracks SET play_count = play_count + 1, last_played = ?2 WHERE id = ?1",
            params![id, now()],
        )?;
        Ok(())
    }

    /// Sets a 0-5 star rating.
    pub fn set_rating(&self, id: i64, rating: u8) -> Result<()> {
        self.db.execute("UPDATE tracks SET rating = ?2 WHERE id = ?1", params![id, rating.min(5)])?;
        Ok(())
    }
}

/// Stores a thumbnail of `data` in the art cache (once per distinct image)
/// and returns its key.
fn cache_art(dir: &Path, data: &[u8]) -> std::result::Result<String, String> {
    use sha1::{Digest, Sha1};
    let key = hex::encode(&Sha1::digest(data)[..10]);
    let out = dir.join(format!("{key}.jpg"));
    if !out.exists() {
        let img = image::load_from_memory(data).map_err(|e| e.to_string())?;
        let img = img.resize(ART_SIZE, ART_SIZE, image::imageops::FilterType::Lanczos3).to_rgb8();
        let tmp = out.with_extension("tmp");
        let mut f = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut f, 88)
            .encode_image(&img)
            .map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &out).map_err(|e| e.to_string())?;
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_library_and_folders() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::open(&dir.path().join("lib.db"), &dir.path().join("art")).unwrap();
        let music = dir.path().join("music");
        std::fs::create_dir(&music).unwrap();
        std::fs::write(music.join("notes.txt"), "not audio").unwrap();
        let f = lib.add_folder(&music).unwrap();
        assert_eq!(lib.add_folder(&music).unwrap(), f, "adding twice is a no-op");
        let r = lib.scan(&mut |_| {}).unwrap();
        assert_eq!((r.seen, r.added), (0, 0));
        assert!(lib.albums(None).unwrap().is_empty());
        lib.remove_folder(f.id).unwrap();
        assert!(lib.folders().unwrap().is_empty());
    }

    #[test]
    fn unreadable_audio_is_reported_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::open(&dir.path().join("lib.db"), &dir.path().join("art")).unwrap();
        let music = dir.path().join("music");
        std::fs::create_dir(&music).unwrap();
        std::fs::write(music.join("broken.mp3"), b"definitely not an mp3").unwrap();
        lib.add_folder(&music).unwrap();
        let r = lib.scan(&mut |_| {}).unwrap();
        assert_eq!(r.seen, 1);
        assert_eq!(r.errors.len(), 1);
    }
}
