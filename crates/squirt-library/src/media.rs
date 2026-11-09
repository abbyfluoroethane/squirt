//! Videos and pictures: scanning and queries.
//!
//! Video details (duration, size, codec) and thumbnails come from ffprobe and
//! ffmpeg when they are installed; without them videos are still listed, with
//! no duration or thumbnail. Pictures are read with the `image` crate.

use crate::{file_mtime, now_unix, Library, Result};
use rusqlite::{params, OptionalExtension, Transaction};
use std::path::{Path, PathBuf};
use std::process::Command;

pub const VIDEO_EXTENSIONS: &[&str] = &["mp4", "m4v", "mov", "wmv", "avi", "mkv", "webm"];
pub const PICTURE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "gif", "bmp"];

/// Longest edge of video and picture thumbnails.
const THUMB: u32 = 480;

#[derive(Debug, Clone)]
pub struct Video {
    pub id: i64,
    pub path: PathBuf,
    pub title: String,
    pub size: u64,
    pub duration_ms: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codec: Option<String>,
    pub thumb: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Picture {
    pub id: i64,
    pub path: PathBuf,
    pub album: String,
    pub size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub thumb: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PictureAlbum {
    pub name: String,
    pub count: u32,
    /// Thumbnail of the first picture.
    pub cover: Option<String>,
}

/// Finds ffmpeg/ffprobe. Releases bundle Squirt's own minimal LGPL build as
/// a sidecar next to the app's executable, so that comes first; then PATH,
/// then the usual install locations (apps started from the Dock on macOS
/// don't get Homebrew's PATH).
pub fn find_tool(name: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        dirs.push(dir);
    }
    dirs.extend(std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect::<Vec<_>>()).unwrap_or_default());
    dirs.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].map(PathBuf::from));
    let exe = if cfg!(windows) { format!("{name}.exe") } else { name.to_string() };
    dirs.into_iter().map(|d| d.join(&exe)).find(|p| p.is_file())
}

fn thumb_key(path: &Path, mtime: i64) -> String {
    use sha1::{Digest, Sha1};
    let mut h = Sha1::new();
    h.update(path.to_string_lossy().as_bytes());
    h.update(mtime.to_le_bytes());
    format!("t{}", hex::encode(&h.finalize()[..9]))
}

/// Size and mtime unchanged since the last scan?
fn unchanged(tx: &Transaction, table: &str, key: &str, size: i64, mtime: i64) -> rusqlite::Result<Option<bool>> {
    let known: Option<(i64, i64)> = tx
        .query_row(&format!("SELECT size, mtime FROM {table} WHERE path = ?1"), [key], |r| Ok((r.get(0)?, r.get(1)?)))
        .optional()?;
    Ok(match known {
        Some(k) if k == (size, mtime) => Some(true),
        Some(_) => Some(false),
        None => None,
    })
}

struct Probe {
    duration_ms: u64,
    width: Option<u32>,
    height: Option<u32>,
    codec: Option<String>,
    title: Option<String>,
}

fn ffprobe(path: &Path) -> Option<Probe> {
    let tool = find_tool("ffprobe")?;
    let out = Command::new(tool)
        .args(["-v", "error", "-print_format", "json", "-show_format", "-show_streams"])
        .arg(path)
        .output()
        .ok()?;
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).ok()?;
    let video = v["streams"].as_array()?.iter().find(|s| s["codec_type"] == "video");
    Some(Probe {
        duration_ms: v["format"]["duration"].as_str().and_then(|d| d.parse::<f64>().ok()).map(|d| (d * 1000.0) as u64).unwrap_or(0),
        width: video.and_then(|s| s["width"].as_u64()).map(|w| w as u32),
        height: video.and_then(|s| s["height"].as_u64()).map(|h| h as u32),
        codec: video.and_then(|s| s["codec_name"].as_str()).map(str::to_owned),
        title: v["format"]["tags"]["title"].as_str().filter(|t| !t.trim().is_empty()).map(str::to_owned),
    })
}

/// A frame from 10% into the video, scaled to fit THUMB, as JPEG.
fn video_thumb(path: &Path, art_dir: &Path, key: &str, duration_ms: u64) -> Option<String> {
    let out = art_dir.join(format!("{key}.jpg"));
    if out.exists() {
        return Some(key.into());
    }
    let tool = find_tool("ffmpeg")?;
    let at = format!("{:.2}", (duration_ms as f64 / 1000.0 * 0.1).max(0.0));
    let ok = Command::new(tool)
        .args(["-v", "error", "-y", "-ss", &at, "-i"])
        .arg(path)
        .args(["-frames:v", "1", "-vf", &format!("scale='min({THUMB},iw)':-2"), "-q:v", "4"])
        .arg(&out)
        .status()
        .ok()?
        .success();
    (ok && out.exists()).then(|| key.into())
}

fn picture_thumb(path: &Path, art_dir: &Path, key: &str) -> Option<String> {
    let out = art_dir.join(format!("{key}.jpg"));
    if !out.exists() {
        let img = image::open(path).ok()?;
        let img = img.thumbnail(THUMB, THUMB).to_rgb8();
        let mut f = std::fs::File::create(&out).ok()?;
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut f, 82).encode_image(&img).ok()?;
    }
    Some(key.into())
}

/// Scans one video. Returns Some(true) if added, Some(false) if updated, None if unchanged.
pub(crate) fn scan_video(tx: &Transaction, art_dir: &Path, folder_id: i64, path: &Path) -> std::result::Result<Option<bool>, String> {
    let md = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let (size, mtime) = (md.len() as i64, file_mtime(&md));
    let key = path.to_string_lossy().into_owned();
    let state = unchanged(tx, "videos", &key, size, mtime).map_err(|e| e.to_string())?;
    if state == Some(true) {
        return Ok(None);
    }
    let probe = ffprobe(path);
    let duration_ms = probe.as_ref().map_or(0, |p| p.duration_ms);
    let title = probe
        .as_ref()
        .and_then(|p| p.title.clone())
        .unwrap_or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default());
    let thumb = video_thumb(path, art_dir, &thumb_key(path, mtime), duration_ms);
    tx.execute(
        "INSERT INTO videos (path, folder_id, size, mtime, title, duration_ms, width, height, codec, thumb, added_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(path) DO UPDATE SET folder_id = excluded.folder_id, size = excluded.size, mtime = excluded.mtime,
            title = excluded.title, duration_ms = excluded.duration_ms, width = excluded.width,
            height = excluded.height, codec = excluded.codec, thumb = excluded.thumb",
        params![
            key, folder_id, size, mtime, title, duration_ms as i64,
            probe.as_ref().and_then(|p| p.width), probe.as_ref().and_then(|p| p.height),
            probe.as_ref().and_then(|p| p.codec.clone()), thumb, now_unix()
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(Some(state.is_none()))
}

/// Scans one picture. The album is its folder, relative to the watched root
/// (pictures directly in the root go in an album named after the root).
pub(crate) fn scan_picture(tx: &Transaction, art_dir: &Path, folder_id: i64, path: &Path, root: &Path) -> std::result::Result<Option<bool>, String> {
    let md = std::fs::metadata(path).map_err(|e| e.to_string())?;
    let (size, mtime) = (md.len() as i64, file_mtime(&md));
    let key = path.to_string_lossy().into_owned();
    let state = unchanged(tx, "pictures", &key, size, mtime).map_err(|e| e.to_string())?;
    if state == Some(true) {
        return Ok(None);
    }
    let (width, height) = image::image_dimensions(path).map_err(|e| e.to_string())?;
    let parent = path.parent().unwrap_or(root);
    let album = match parent.strip_prefix(root) {
        Ok(rel) if !rel.as_os_str().is_empty() => rel.to_string_lossy().replace(std::path::MAIN_SEPARATOR, " / "),
        _ => root.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "pictures".into()),
    };
    let thumb = picture_thumb(path, art_dir, &thumb_key(path, mtime));
    tx.execute(
        "INSERT INTO pictures (path, folder_id, size, mtime, album, width, height, thumb, added_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(path) DO UPDATE SET folder_id = excluded.folder_id, size = excluded.size, mtime = excluded.mtime,
            album = excluded.album, width = excluded.width, height = excluded.height, thumb = excluded.thumb",
        params![key, folder_id, size, mtime, album, width, height, thumb, now_unix()],
    )
    .map_err(|e| e.to_string())?;
    Ok(Some(state.is_none()))
}

impl Library {
    pub fn videos(&self) -> Result<Vec<Video>> {
        let mut st = self.db.prepare(
            "SELECT id, path, title, size, duration_ms, width, height, codec, thumb FROM videos ORDER BY title COLLATE NOCASE",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Video {
                id: r.get(0)?,
                path: PathBuf::from(r.get::<_, String>(1)?),
                title: r.get(2)?,
                size: r.get::<_, i64>(3)? as u64,
                duration_ms: r.get::<_, i64>(4)? as u64,
                width: r.get(5)?,
                height: r.get(6)?,
                codec: r.get(7)?,
                thumb: r.get(8)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn video(&self, id: i64) -> Result<Option<Video>> {
        Ok(self.videos()?.into_iter().find(|v| v.id == id))
    }

    pub fn picture_albums(&self) -> Result<Vec<PictureAlbum>> {
        let mut st = self.db.prepare(
            "SELECT album, COUNT(*), (SELECT thumb FROM pictures p2 WHERE p2.album = p.album ORDER BY path LIMIT 1)
             FROM pictures p GROUP BY album ORDER BY album COLLATE NOCASE",
        )?;
        let rows = st.query_map([], |r| Ok(PictureAlbum { name: r.get(0)?, count: r.get(1)?, cover: r.get(2)? }))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn pictures(&self, album: &str) -> Result<Vec<Picture>> {
        let mut st = self.db.prepare(
            "SELECT id, path, album, size, width, height, thumb FROM pictures WHERE album = ?1 ORDER BY path",
        )?;
        let rows = st.query_map([album], |r| {
            Ok(Picture {
                id: r.get(0)?,
                path: PathBuf::from(r.get::<_, String>(1)?),
                album: r.get(2)?,
                size: r.get::<_, i64>(3)? as u64,
                width: r.get(4)?,
                height: r.get(5)?,
                thumb: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Path of a picture or video file by id, for viewing or playback.
    pub fn media_path(&self, table: &str, id: i64) -> Result<Option<PathBuf>> {
        let table = if table == "videos" { "videos" } else { "pictures" };
        Ok(self
            .db
            .query_row(&format!("SELECT path FROM {table} WHERE id = ?1"), [id], |r| r.get::<_, String>(0))
            .optional()?
            .map(PathBuf::from))
    }
}
