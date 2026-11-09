//! Squirt's media library: a SQLite index of the folders the user watches,
//! one set of folders per kind of media (music, podcasts, videos, pictures).
//!
//! Files are never moved or changed by scanning. A scan reads tags, audio
//! and video properties, and art; only files whose size or modification time
//! changed since the last scan are read again. Cover art and thumbnails are
//! stored in a cache directory as JPEGs.

mod media;
mod meta;

pub use media::{find_tool, Picture, PictureAlbum, Video, PICTURE_EXTENSIONS, VIDEO_EXTENSIONS};
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

/// What a watched folder holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Music,
    Podcast,
    Video,
    Picture,
}

impl Kind {
    pub const ALL: [Kind; 4] = [Kind::Music, Kind::Podcast, Kind::Video, Kind::Picture];

    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Music => "music",
            Kind::Podcast => "podcast",
            Kind::Video => "video",
            Kind::Picture => "picture",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Kind::ALL.into_iter().find(|k| k.as_str() == s)
    }

    fn matches(self, p: &Path) -> bool {
        let exts: &[&str] = match self {
            Kind::Music | Kind::Podcast => AUDIO_EXTENSIONS,
            Kind::Video => VIDEO_EXTENSIONS,
            Kind::Picture => PICTURE_EXTENSIONS,
        };
        p.extension().is_some_and(|e| exts.contains(&e.to_string_lossy().to_lowercase().as_str()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Folder {
    pub id: i64,
    pub path: PathBuf,
    pub kind: Kind,
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
    /// Release date, "YYYY-MM-DD" or "YYYY" (podcast episodes use this).
    pub date: Option<String>,
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
    pub(crate) db: Connection,
    pub(crate) art_dir: PathBuf,
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
CREATE TABLE IF NOT EXISTS videos (
    id          INTEGER PRIMARY KEY,
    path        TEXT NOT NULL UNIQUE,
    folder_id   INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    size        INTEGER NOT NULL,
    mtime       INTEGER NOT NULL,
    title       TEXT NOT NULL,
    duration_ms INTEGER NOT NULL,
    width       INTEGER,
    height      INTEGER,
    codec       TEXT,
    thumb       TEXT,
    added_at    INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS pictures (
    id        INTEGER PRIMARY KEY,
    path      TEXT NOT NULL UNIQUE,
    folder_id INTEGER NOT NULL REFERENCES folders(id) ON DELETE CASCADE,
    size      INTEGER NOT NULL,
    mtime     INTEGER NOT NULL,
    album     TEXT NOT NULL,
    width     INTEGER,
    height    INTEGER,
    thumb     TEXT,
    added_at  INTEGER NOT NULL
);
"#;

/// Columns added after the first release, applied to existing databases.
const MIGRATIONS: &[(&str, &str, &str)] = &[
    ("folders", "kind", "TEXT NOT NULL DEFAULT 'music'"),
    ("tracks", "kind", "TEXT NOT NULL DEFAULT 'music'"),
    ("tracks", "date", "TEXT"),
];

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

pub(crate) fn now_unix() -> i64 {
    now()
}

pub(crate) fn file_mtime(md: &std::fs::Metadata) -> i64 {
    mtime_of(md)
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
        for (table, column, def) in MIGRATIONS {
            let exists: bool = db
                .prepare(&format!("SELECT 1 FROM pragma_table_info('{table}') WHERE name = ?1"))?
                .exists([column])?;
            if !exists {
                db.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {def}"))?;
            }
        }
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

    /// Watches a folder for one kind of media. Adding a folder that is already
    /// watched changes its kind.
    pub fn add_folder(&self, path: &Path, kind: Kind) -> Result<Folder> {
        let path = path.canonicalize()?;
        if !path.is_dir() {
            return Err(LibraryError::NotAFolder(path));
        }
        let s = path.to_string_lossy();
        self.db.execute(
            "INSERT INTO folders(path, kind) VALUES (?1, ?2) ON CONFLICT(path) DO UPDATE SET kind = excluded.kind",
            params![&*s, kind.as_str()],
        )?;
        let id = self.db.query_row("SELECT id FROM folders WHERE path = ?1", [&*s], |r| r.get(0))?;
        Ok(Folder { id, path, kind })
    }

    /// Stops watching a folder and forgets its tracks (files are untouched).
    pub fn remove_folder(&self, id: i64) -> Result<()> {
        self.db.execute("DELETE FROM folders WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn folders(&self) -> Result<Vec<Folder>> {
        let mut st = self.db.prepare("SELECT id, path, kind FROM folders ORDER BY kind, path")?;
        let rows = st.query_map([], |r| {
            Ok(Folder {
                id: r.get(0)?,
                path: PathBuf::from(r.get::<_, String>(1)?),
                kind: Kind::parse(&r.get::<_, String>(2)?).unwrap_or(Kind::Music),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // --------------------------------------------------------------- scan

    /// Scans every watched folder: adds new files, re-reads changed ones and
    /// forgets files that are gone.
    pub fn scan(&mut self, progress: &mut dyn FnMut(ScanProgress)) -> Result<ScanReport> {
        let mut report = ScanReport::default();
        let folders = self.folders()?;

        // Audio (music and podcasts) goes through the loop below; videos and
        // pictures have their own scanners in `media`.
        let mut files: Vec<(i64, Kind, PathBuf)> = Vec::new();
        let mut media_files: Vec<(i64, Kind, PathBuf, PathBuf)> = Vec::new();
        for f in &folders {
            for entry in walkdir::WalkDir::new(&f.path).follow_links(true).into_iter().filter_map(|e| e.ok()) {
                let p = entry.path();
                let hidden = entry.file_name().to_string_lossy().starts_with('.');
                if !entry.file_type().is_file() || hidden || !f.kind.matches(p) {
                    continue;
                }
                match f.kind {
                    Kind::Music | Kind::Podcast => files.push((f.id, f.kind, p.to_path_buf())),
                    Kind::Video | Kind::Picture => media_files.push((f.id, f.kind, p.to_path_buf(), f.path.clone())),
                }
            }
        }
        report.seen = files.len() + media_files.len();

        let mut present: HashSet<String> = HashSet::new();
        let total = files.len();
        let tx = self.db.transaction()?;
        let total = total + media_files.len();
        for (i, (folder_id, kind, path)) in files.iter().enumerate() {
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
                    track_no, track_total, disc_no, disc_total, duration_ms, codec, bitrate_kbps, art, added_at, kind, date)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
                 ON CONFLICT(path) DO UPDATE SET kind = excluded.kind, date = excluded.date,
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
                    meta.duration_ms as i64, meta.codec.as_str(), meta.bitrate_kbps, art, now(),
                    kind.as_str(), meta.date
                ],
            )?;
            if known.is_some() {
                report.updated += 1;
            } else {
                report.added += 1;
            }
        }

        // Videos and pictures.
        let offset = files.len();
        for (i, (folder_id, kind, path, root)) in media_files.iter().enumerate() {
            progress(ScanProgress { done: offset + i, total, path });
            present.insert(path.to_string_lossy().into_owned());
            let result = match kind {
                Kind::Video => media::scan_video(&tx, &self.art_dir, *folder_id, path),
                _ => media::scan_picture(&tx, &self.art_dir, *folder_id, path, root),
            };
            match result {
                Ok(Some(true)) => report.added += 1,
                Ok(Some(false)) => report.updated += 1,
                Ok(None) => {}
                Err(e) => report.errors.push((path.clone(), e)),
            }
        }
        for table in ["videos", "pictures"] {
            let gone: Vec<i64> = {
                let mut st = tx.prepare(&format!("SELECT id, path FROM {table}"))?;
                let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?;
                rows.collect::<rusqlite::Result<Vec<_>>>()?.into_iter().filter(|(_, p)| !present.contains(p)).map(|(id, _)| id).collect()
            };
            for id in &gone {
                tx.execute(&format!("DELETE FROM {table} WHERE id = ?1"), [id])?;
            }
            report.removed += gone.len();
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
        report.removed += gone.len();
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
            .prepare("SELECT DISTINCT album_artist FROM tracks WHERE kind = 'music' ORDER BY album_artist COLLATE NOCASE")?;
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
             WHERE kind = 'music' AND (?1 IS NULL OR album_artist = ?1)
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
        self.tracks_where("kind = 'music' AND album_artist = ?1 AND album = ?2", params![artist, album])
    }

    pub fn track(&self, id: i64) -> Result<Option<Track>> {
        Ok(self.tracks_where("id = ?1", params![id])?.into_iter().next())
    }

    fn tracks_where(&self, cond: &str, p: impl rusqlite::Params) -> Result<Vec<Track>> {
        let sql = format!(
            "SELECT id, path, size, title, artist, album, album_artist, genre, year, track_no, disc_no, disc_total,
                    duration_ms, codec, bitrate_kbps, art, rating, play_count, date
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
                date: r.get(18)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Podcast shows (album = show), newest episode first within each.
    pub fn podcast_shows(&self) -> Result<Vec<Album>> {
        let mut st = self.db.prepare(
            "SELECT album, album_artist, MAX(year), MAX(genre), COUNT(*), SUM(duration_ms), SUM(size),
                    (SELECT art FROM tracks t2 WHERE t2.kind = 'podcast' AND t2.album = t.album AND art IS NOT NULL LIMIT 1)
             FROM tracks t WHERE kind = 'podcast'
             GROUP BY album ORDER BY album COLLATE NOCASE",
        )?;
        let rows = st.query_map([], |r| {
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

    /// Episodes of a show, newest first.
    pub fn podcast_episodes(&self, show: &str) -> Result<Vec<Track>> {
        let mut v = self.tracks_where("kind = 'podcast' AND album = ?1", params![show])?;
        v.sort_by(|a, b| b.date.cmp(&a.date).then(b.track_no.cmp(&a.track_no)));
        Ok(v)
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
        let f = lib.add_folder(&music, Kind::Music).unwrap();
        assert_eq!(lib.add_folder(&music, Kind::Music).unwrap(), f, "adding twice is a no-op");
        let r = lib.scan(&mut |_| {}).unwrap();
        assert_eq!((r.seen, r.added), (0, 0));
        assert!(lib.albums(None).unwrap().is_empty());
        lib.remove_folder(f.id).unwrap();
        assert!(lib.folders().unwrap().is_empty());
    }

    #[test]
    fn scans_pictures_into_albums_and_videos() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::open(&dir.path().join("lib.db"), &dir.path().join("art")).unwrap();
        let pics = dir.path().join("Pictures");
        std::fs::create_dir_all(pics.join("Trip")).unwrap();
        let img = image::RgbImage::from_fn(64, 48, |x, y| image::Rgb([x as u8 * 4, y as u8 * 5, 128]));
        img.save(pics.join("a.png")).unwrap();
        img.save(pics.join("Trip").join("b.jpg")).unwrap();
        lib.add_folder(&pics, Kind::Picture).unwrap();

        // A one-second test video, when ffmpeg is available.
        let vids = dir.path().join("Movies");
        std::fs::create_dir_all(&vids).unwrap();
        let have_ffmpeg = media::find_tool("ffmpeg").is_some_and(|ff| {
            std::process::Command::new(ff)
                .args(["-v", "error", "-f", "lavfi", "-i", "testsrc=size=320x240:duration=1", "-pix_fmt", "yuv420p"])
                .arg(vids.join("clip.mp4"))
                .status()
                .is_ok_and(|s| s.success())
        });
        lib.add_folder(&vids, Kind::Video).unwrap();

        let r = lib.scan(&mut |_| {}).unwrap();
        assert!(r.errors.is_empty(), "{:?}", r.errors);
        let albums = lib.picture_albums().unwrap();
        let names: Vec<_> = albums.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Pictures", "Trip"]);
        let p = &lib.pictures("Trip").unwrap()[0];
        assert_eq!((p.width, p.height), (Some(64), Some(48)));
        assert!(p.thumb.as_ref().is_some_and(|k| lib.art_path(k).exists()));

        if have_ffmpeg {
            let v = &lib.videos().unwrap()[0];
            assert_eq!(v.title, "clip");
            assert_eq!((v.width, v.height), (Some(320), Some(240)));
            assert!((900..=1100).contains(&v.duration_ms), "{}", v.duration_ms);
            assert!(v.thumb.as_ref().is_some_and(|k| lib.art_path(k).exists()));
        }

        // A second scan reads nothing again; removing a file forgets it.
        assert_eq!(lib.scan(&mut |_| {}).unwrap().added, 0);
        std::fs::remove_file(pics.join("a.png")).unwrap();
        assert_eq!(lib.scan(&mut |_| {}).unwrap().removed, 1);
    }

    #[test]
    fn unreadable_audio_is_reported_not_fatal() {
        let dir = tempfile::tempdir().unwrap();
        let mut lib = Library::open(&dir.path().join("lib.db"), &dir.path().join("art")).unwrap();
        let music = dir.path().join("music");
        std::fs::create_dir(&music).unwrap();
        std::fs::write(music.join("broken.mp3"), b"definitely not an mp3").unwrap();
        lib.add_folder(&music, Kind::Music).unwrap();
        let r = lib.scan(&mut |_| {}).unwrap();
        assert_eq!(r.seen, 1);
        assert_eq!(r.errors.len(), 1);
    }
}
