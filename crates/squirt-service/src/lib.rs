//! The app's backend: library, player and Zune, behind one thread-safe API.
//!
//! Both front ends use this: the Tauri app (as commands) and `squirt serve`
//! (as a JSON API for developing the UI in a browser). Each part has its own
//! lock, so a slow device read never blocks playback controls.

use serde::Serialize;
use squirt_library::Library;
use squirt_mtp::device::Zune;
use squirt_mtp::library::{DeviceAlbum, DeviceTrack};
use squirt_mtp::mtpz::MtpzKeys;
use squirt_player::{Event, Player, Status};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub mod push;
mod setup;
mod sync;
mod social;
mod watch;
pub use setup::{SetupStatus, SuggestedFolder};
pub use sync::SyncStatus;
pub use social::{LastFmProfile, ScrobbleDto, SocialStatus};

pub type Result<T> = std::result::Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

// ------------------------------------------------------------------ types

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrackDto {
    pub id: i64,
    pub disc: u32,
    pub n: u32,
    pub title: String,
    pub artist: String,
    pub duration_ms: u64,
    pub size: u64,
    pub codec: String,
    pub on_device: bool,
    /// Release date (podcast episodes show this).
    pub date: Option<String>,
    /// The file on this computer (none for songs only on the device).
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbumDto {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub hue: u32,
    /// Art key: `lib/<key>` for the library cache, `dev/<handle>` for device art.
    pub art: Option<String>,
    pub tracks: Vec<TrackDto>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Usage {
    pub music: u64,
    pub videos: u64,
    pub pictures: u64,
    pub other: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub connected: bool,
    pub name: String,
    pub model: String,
    pub firmware: String,
    pub capacity: u64,
    pub free: u64,
    pub usage: Usage,
    /// Set when a Zune is plugged in but cannot be used (no keys, handshake failed).
    pub problem: Option<String>,
    /// "slate" or "platinum", from the device's own icon; None if unknown.
    pub finish: Option<String>,
    /// Which body to draw: "zunehd", "zune30", "zuneflash" or "zunehdd".
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerStateDto {
    pub status: &'static str,
    pub track_id: Option<i64>,
    pub position_ms: u64,
    pub duration_ms: u64,
    pub volume: f32,
    pub queue: Vec<i64>,
}

/// What the OS "now playing" panel shows for a library track.
#[derive(Debug, Clone)]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub duration_ms: u64,
    /// Cover art as a JPEG file on disk, if the track has art.
    pub art_path: Option<PathBuf>,
}

// ----------------------------------------------------------------- device

/// The device's music, read once per connection (it takes a few seconds).
struct DeviceLibrary {
    tracks: Vec<DeviceTrack>,
    albums: Vec<DeviceAlbum>,
}

#[derive(Default)]
struct DeviceSlot {
    /// Set by "disconnect": stay off the device until it is unplugged or the
    /// user connects again.
    released: bool,
    zune: Option<Zune>,
    storage: u32,
    finish: Option<String>,
    status: DeviceStatus,
    checked: Option<Instant>,
    library: Option<DeviceLibrary>,
    art: HashMap<u32, Vec<u8>>,
}

fn hue(s: &str) -> u32 {
    s.chars().fold(0u32, |h, c| h.wrapping_mul(31).wrapping_add(c as u32)) % 360
}

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Everything the device info page shows.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeviceInfoDto {
    pub connected: bool,
    /// True after "disconnect", until replugged or reconnected.
    pub released: bool,
    pub name: String,
    pub manufacturer: String,
    pub model: String,
    pub firmware: String,
    pub serial: String,
    pub battery: Option<u8>,
    pub capacity: u64,
    pub free: u64,
    pub usage: Usage,
    pub tracks: usize,
    pub albums: usize,
    pub artists: usize,
    /// "slate" or "platinum"; None if unknown.
    pub finish: Option<String>,
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderDto {
    pub id: i64,
    pub path: String,
    pub kind: &'static str,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScanStatus {
    pub running: bool,
    pub done: usize,
    pub total: usize,
    /// Summary of the last finished scan.
    pub last: Option<String>,
    pub errors: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoDto {
    pub id: i64,
    pub title: String,
    pub path: String,
    pub size: u64,
    pub duration_ms: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub codec: Option<String>,
    /// Art key (`lib/<key>`).
    pub thumb: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PictureDto {
    pub id: i64,
    pub album: String,
    pub path: String,
    pub size: u64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub thumb: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PictureAlbumDto {
    pub name: String,
    pub count: u32,
    pub cover: Option<String>,
    pub pictures: Vec<PictureDto>,
}

// ---------------------------------------------------------------- service

struct Playback {
    player: Player,
    queue: Vec<i64>,
}

pub struct Service {
    scan: Mutex<ScanStatus>,
    /// A rescan was asked for while one was running; run again after it.
    scan_again: std::sync::atomic::AtomicBool,
    lib: Mutex<Library>,
    player: Mutex<Option<Playback>>,
    device: Mutex<DeviceSlot>,
    /// (album artist, album, title) of every song on the device. Kept apart
    /// from `device` so the collection never waits on a slow device read.
    on_device: Mutex<Arc<HashSet<(String, String, String)>>>,
    sync: Mutex<sync::SyncState>,
    social: Mutex<social::SocialState>,
}

impl Service {
    pub fn open() -> Result<Self> {
        let (db, art) = Library::default_dirs().ok_or("no data directory")?;
        let social_path = db.with_file_name("social.json");
        Ok(Self {
            scan: Mutex::new(ScanStatus::default()),
            scan_again: Default::default(),
            lib: Mutex::new(Library::open(&db, &art).map_err(err)?),
            player: Mutex::new(None),
            device: Mutex::new(DeviceSlot::default()),
            on_device: Mutex::new(Arc::default()),
            sync: Mutex::new(Default::default()),
            social: Mutex::new(social::SocialState::load(social_path)),
        })
    }

    // --------------------------------------------------------- library

    /// Every album in the library, with tracks, marking the ones on the device.
    pub fn collection(&self) -> Result<Vec<AlbumDto>> {
        let on_device = self.on_device.lock().unwrap().clone();
        let lib = self.lib.lock().unwrap();
        let mut out = Vec::new();
        for a in lib.albums(None).map_err(err)? {
            let tracks = lib
                .album_tracks(&a.artist, &a.title)
                .map_err(err)?
                .into_iter()
                .map(|t| TrackDto {
                    on_device: on_device.contains(&(norm(&t.album_artist), norm(&t.album), norm(&t.title))),
                    path: Some(t.path.to_string_lossy().into_owned()),
                    id: t.id,
                    disc: t.disc_no.unwrap_or(1),
                    n: t.track_no.unwrap_or(0),
                    title: t.title,
                    artist: t.artist,
                    duration_ms: t.duration_ms,
                    size: t.size,
                    codec: t.codec,
                    date: t.date,
                })
                .collect();
            out.push(AlbumDto {
                id: format!("{}\u{1f}{}", a.artist, a.title),
                hue: hue(&format!("{}{}", a.artist, a.title)),
                art: a.art.map(|k| format!("lib/{k}")),
                title: a.title,
                artist: a.artist,
                year: a.year,
                genre: a.genre,
                tracks,
            });
        }
        Ok(out)
    }

    // --------------------------------------------------------- folders

    pub fn folders(&self) -> Result<Vec<FolderDto>> {
        Ok(self
            .lib
            .lock()
            .unwrap()
            .folders()
            .map_err(err)?
            .into_iter()
            .map(|f| FolderDto { id: f.id, path: f.path.to_string_lossy().into_owned(), kind: f.kind.as_str() })
            .collect())
    }

    /// Watches a folder for a kind of media ("music", "podcast", "video",
    /// "picture"), then rescans in the background.
    pub fn add_folder(self: &Arc<Self>, path: &str, kind: &str) -> Result<Vec<FolderDto>> {
        let kind = squirt_library::Kind::parse(kind).ok_or("unknown media kind")?;
        let path = expand_home(path);
        self.lib.lock().unwrap().add_folder(&path, kind).map_err(err)?;
        self.rescan();
        self.folders()
    }

    /// Stops watching a folder; its files are forgotten (not deleted).
    pub fn remove_folder(&self, id: i64) -> Result<Vec<FolderDto>> {
        self.lib.lock().unwrap().remove_folder(id).map_err(err)?;
        self.folders()
    }

    pub fn scan_status(&self) -> ScanStatus {
        self.scan.lock().unwrap().clone()
    }

    /// Rescans every watched folder in the background, on its own database
    /// connection so browsing isn't blocked. Does nothing if one is running.
    pub fn rescan(self: &Arc<Self>) {
        {
            let mut s = self.scan.lock().unwrap();
            if s.running {
                // Folders may have changed since that scan read them.
                self.scan_again.store(true, std::sync::atomic::Ordering::SeqCst);
                return;
            }
            *s = ScanStatus { running: true, last: s.last.clone(), ..Default::default() };
        }
        let svc = self.clone();
        std::thread::Builder::new()
            .name("squirt-scan".into())
            .spawn(move || {
                let result = loop {
                    let r = Library::default_dirs()
                        .ok_or_else(|| "no data directory".to_string())
                        .and_then(|(db, art)| Library::open(&db, &art).map_err(err))
                        .and_then(|mut lib| {
                            lib.scan(&mut |p| {
                                let mut s = svc.scan.lock().unwrap();
                                s.done = p.done;
                                s.total = p.total;
                            })
                            .map_err(err)
                        });
                    if !svc.scan_again.swap(false, std::sync::atomic::Ordering::SeqCst) {
                        break r;
                    }
                };
                let mut s = svc.scan.lock().unwrap();
                s.running = false;
                match result {
                    Ok(r) => {
                        s.done = r.seen;
                        s.total = r.seen;
                        s.errors = r.errors.len();
                        s.last = Some(format!("{} files: {} new, {} changed, {} removed", r.seen, r.added, r.updated, r.removed));
                    }
                    Err(e) => s.last = Some(format!("scan failed: {e}")),
                }
            })
            .expect("spawn scan thread");
    }

    // ---------------------------------------------------- other media

    pub fn videos(&self) -> Result<Vec<VideoDto>> {
        Ok(self
            .lib
            .lock()
            .unwrap()
            .videos()
            .map_err(err)?
            .into_iter()
            .map(|v| VideoDto {
                id: v.id,
                title: v.title,
                path: v.path.to_string_lossy().into_owned(),
                size: v.size,
                duration_ms: v.duration_ms,
                width: v.width,
                height: v.height,
                codec: v.codec,
                thumb: v.thumb.map(|k| format!("lib/{k}")),
            })
            .collect())
    }

    pub fn pictures(&self) -> Result<Vec<PictureAlbumDto>> {
        let lib = self.lib.lock().unwrap();
        let mut out = Vec::new();
        for a in lib.picture_albums().map_err(err)? {
            let pictures = lib
                .pictures(&a.name)
                .map_err(err)?
                .into_iter()
                .map(|p| PictureDto {
                    id: p.id,
                    album: p.album,
                    path: p.path.to_string_lossy().into_owned(),
                    size: p.size,
                    width: p.width,
                    height: p.height,
                    thumb: p.thumb.map(|k| format!("lib/{k}")),
                })
                .collect();
            out.push(PictureAlbumDto { name: a.name, count: a.count, cover: a.cover.map(|k| format!("lib/{k}")), pictures });
        }
        Ok(out)
    }

    /// Podcast shows as albums; episodes newest first.
    pub fn podcasts(&self) -> Result<Vec<AlbumDto>> {
        let lib = self.lib.lock().unwrap();
        let mut out = Vec::new();
        for show in lib.podcast_shows().map_err(err)? {
            let tracks = lib
                .podcast_episodes(&show.title)
                .map_err(err)?
                .into_iter()
                .enumerate()
                .map(|(i, t)| TrackDto {
                    path: Some(t.path.to_string_lossy().into_owned()),
                    id: t.id,
                    disc: 1,
                    n: (i + 1) as u32,
                    title: t.title,
                    artist: t.artist,
                    duration_ms: t.duration_ms,
                    size: t.size,
                    codec: t.codec,
                    on_device: false,
                    date: t.date,
                })
                .collect();
            out.push(AlbumDto {
                id: format!("podcast\u{1f}{}", show.title),
                hue: hue(&show.title),
                art: show.art.map(|k| format!("lib/{k}")),
                title: show.title,
                artist: show.artist,
                year: show.year,
                genre: show.genre,
                tracks,
            });
        }
        Ok(out)
    }

    /// File path of a video or picture, for playback/viewing.
    pub fn media_path(&self, kind: &str, id: i64) -> Option<PathBuf> {
        let table = if kind == "video" { "videos" } else { "pictures" };
        self.lib.lock().unwrap().media_path(table, id).ok().flatten()
    }

    /// Bytes of an art image by key (`lib/<key>` or `dev/<handle>`), as JPEG.
    pub fn art(&self, key: &str) -> Option<Vec<u8>> {
        if let Some(k) = key.strip_prefix("lib/") {
            if !k.chars().all(|c| c.is_ascii_hexdigit()) {
                return None;
            }
            let path = self.lib.lock().unwrap().art_path(k);
            return std::fs::read(path).ok();
        }
        let handle: u32 = key.strip_prefix("dev/")?.parse().ok()?;
        let mut dev = self.device.lock().unwrap();
        if let Some(b) = dev.art.get(&handle) {
            return Some(b.clone());
        }
        let bytes = dev.zune.as_mut()?.prop_bytes(handle, squirt_mtp::library::REPRESENTATIVE_SAMPLE_DATA).ok()?;
        dev.art.insert(handle, bytes.clone());
        Some(bytes)
    }

    // ---------------------------------------------------------- device

    /// Device status. Connects when a Zune appears; cheap when nothing changed.
    pub fn device_status(&self) -> DeviceStatus {
        let mut dev = self.device.lock().unwrap();
        if dev.checked.is_some_and(|t| t.elapsed() < Duration::from_secs(2)) {
            return dev.status.clone();
        }
        dev.checked = Some(Instant::now());

        if dev.zune.is_none() {
            let present = squirt_mtp::devices::find_zunes().map(|z| !z.is_empty()).unwrap_or(false);
            if !present {
                // Unplugged: forget a manual disconnect, so replugging connects.
                dev.released = false;
                dev.status = DeviceStatus::default();
                self.set_device_library(&mut dev, None);
                dev.art.clear();
                return dev.status.clone();
            }
            if dev.released {
                dev.status = DeviceStatus::default();
                return dev.status.clone();
            }
            let keys = match MtpzKeys::find() {
                Ok(k) => k,
                Err(e) => return problem(&mut dev, format!("MTPZ keys: {e}")),
            };
            match Zune::connect(Some(&keys)) {
                Ok(mut z) => {
                    dev.storage = z.storage_ids().ok().and_then(|s| s.first().copied()).unwrap_or(0);
                    // Read the device's music now (a few seconds), so the
                    // collection can mark what is already on it.
                    let tracks = z.music_tracks().unwrap_or_default();
                    let albums = z.album_objects(dev.storage).unwrap_or_default();
                    self.set_device_library(&mut dev, Some(DeviceLibrary { tracks, albums }));
                    dev.finish = device_finish(&mut z);
                    dev.zune = Some(z);
                }
                Err(e) => return problem(&mut dev, format!("could not connect: {e}")),
            }
        }

        let storage = dev.storage;
        let z = dev.zune.as_mut().unwrap();
        let info = z.storage_info(storage);
        let name = z
            .raw(0x1015, &[0xD402])
            .ok()
            .and_then(|d| squirt_mtp::dataset::Reader::new(&d).string().ok())
            .unwrap_or_default();
        let model = z.info.model.clone();
        let z_model = z.found.model;
        let firmware = z.info.device_version.split('-').next().unwrap_or("").to_string();
        match info {
            Ok(si) => {
                let music: u64 = dev.library.as_ref().map(|l| l.tracks.iter().map(|t| t.size).sum()).unwrap_or(0);
                let used = si.max_capacity.saturating_sub(si.free_bytes);
                dev.status = DeviceStatus {
                    connected: true,
                    name,
                    model: if model == "Zune" { "Zune HD".into() } else { model },
                    firmware,
                    capacity: si.max_capacity,
                    free: si.free_bytes,
                    usage: Usage { music, other: used.saturating_sub(music), ..Default::default() },
                    problem: None,
                    finish: dev.finish.clone(),
                    kind: Some(device_kind(z_model, si.max_capacity).into()),
                };
            }
            Err(_) => {
                // Unplugged or wedged: drop the connection and report disconnected.
                dev.zune = None;
                self.set_device_library(&mut dev, None);
                dev.art.clear();
                dev.status = DeviceStatus::default();
            }
        }
        dev.status.clone()
    }

    /// Replaces the cached device library and the on-device song keys.
    fn set_device_library(&self, dev: &mut DeviceSlot, lib: Option<DeviceLibrary>) {
        let keys = lib
            .as_ref()
            .map(|l| l.tracks.iter().map(|t| (norm(&t.album_artist), norm(&t.album), norm(&t.title))).collect())
            .unwrap_or_default();
        *self.on_device.lock().unwrap() = Arc::new(keys);
        dev.library = lib;
    }

    /// Details for the device info page.
    pub fn device_info(&self) -> DeviceInfoDto {
        let status = self.device_status();
        let mut dev = self.device.lock().unwrap();
        let released = dev.released;
        let (tracks, albums, artists) = dev
            .library
            .as_ref()
            .map(|l| {
                let artists: HashSet<String> = l.tracks.iter().map(|t| norm(&t.album_artist)).collect();
                (l.tracks.len(), l.albums.len(), artists.len())
            })
            .unwrap_or_default();
        let Some(z) = dev.zune.as_mut() else {
            return DeviceInfoDto { released, ..Default::default() };
        };
        // Battery level (MTP device property 0x5001), 0-100.
        let battery = z.raw(0x1015, &[0x5001]).ok().and_then(|d| d.first().copied()).filter(|&b| b <= 100);
        DeviceInfoDto {
            connected: status.connected,
            released,
            name: status.name,
            manufacturer: z.info.manufacturer.clone(),
            model: status.model,
            firmware: status.firmware,
            serial: z.info.serial_number.clone(),
            battery,
            capacity: status.capacity,
            free: status.free,
            usage: status.usage,
            tracks,
            albums,
            artists,
            finish: status.finish,
            kind: status.kind,
        }
    }

    /// Renames the device (its friendly name, MTP property 0xD402).
    pub fn device_rename(&self, name: &str) -> Result<DeviceInfoDto> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 64 {
            return Err("the name must be 1 to 64 characters".into());
        }
        {
            let mut dev = self.device.lock().unwrap();
            let z = dev.zune.as_mut().ok_or("no device connected")?;
            z.set_device_prop(0xD402, &squirt_mtp::dataset::encode_string(name)).map_err(err)?;
            dev.checked = None; // Re-read the status, including the name.
        }
        Ok(self.device_info())
    }

    /// Re-reads the device's music library.
    pub fn device_refresh(&self) -> Result<DeviceInfoDto> {
        {
            let mut dev = self.device.lock().unwrap();
            let storage = dev.storage;
            let z = dev.zune.as_mut().ok_or("no device connected")?;
            let tracks = z.music_tracks().map_err(err)?;
            let albums = z.album_objects(storage).map_err(err)?;
            self.set_device_library(&mut dev, Some(DeviceLibrary { tracks, albums }));
            dev.art.clear();
            dev.checked = None;
        }
        Ok(self.device_info())
    }

    /// Removes songs from the device. `ids` can mix device song ids
    /// (negative, from the device view) and library ids (positive, matched
    /// to the device by album artist, album and title). Empty album and
    /// artist folders go too. Returns how many songs were removed.
    pub fn device_remove(&self, ids: Vec<i64>) -> Result<usize> {
        if self.sync_status().active {
            return Err("wait for the current sync to finish".into());
        }
        // Library ids -> their (album artist, album, title) keys.
        let keys: HashSet<(String, String, String)> = {
            let lib = self.lib.lock().unwrap();
            ids.iter()
                .filter(|&&id| id > 0)
                .filter_map(|&id| lib.track(id).ok().flatten())
                .map(|t| (norm(&t.album_artist), norm(&t.album), norm(&t.title)))
                .collect()
        };
        let mut dev = self.device.lock().unwrap();
        let storage = dev.storage;
        let library = dev.library.as_ref().ok_or("the zune's library isn't loaded yet")?;
        let mut handles: Vec<u32> = ids.iter().filter(|&&id| id < 0).map(|&id| (-id) as u32).collect();
        for t in &library.tracks {
            if keys.contains(&(norm(&t.album_artist), norm(&t.album), norm(&t.title))) && !handles.contains(&t.handle) {
                handles.push(t.handle);
            }
        }
        let albums: Vec<u32> = {
            let mut a: Vec<u32> = library.tracks.iter().filter(|t| handles.contains(&t.handle)).filter_map(|t| t.album_ref).collect();
            a.sort_unstable();
            a.dedup();
            a
        };
        if handles.is_empty() {
            return Ok(0);
        }
        let z = dev.zune.as_mut().ok_or("no zune connected")?;
        let removed = z.delete_tracks(storage, &handles, &albums).map_err(err);
        // Re-read what's on the device either way (a partial delete is possible).
        let reread = z.music_tracks().and_then(|t| Ok((t, z.album_objects(storage)?)));
        if let Ok((tracks, albums)) = reread {
            self.set_device_library(&mut dev, Some(DeviceLibrary { tracks, albums }));
        }
        dev.art.clear();
        dev.checked = None;
        removed
    }

    /// Lets go of the device (so other tools can use it) until it is
    /// replugged or [`Service::device_connect`] is called.
    pub fn device_release(&self) -> DeviceInfoDto {
        {
            let mut dev = self.device.lock().unwrap();
            dev.zune = None; // Dropping the connection releases the USB interface.
            self.set_device_library(&mut dev, None);
            dev.art.clear();
            dev.released = true;
            dev.checked = None;
        }
        self.device_info()
    }

    pub fn device_connect(&self) -> DeviceInfoDto {
        {
            let mut dev = self.device.lock().unwrap();
            dev.released = false;
            dev.checked = None;
        }
        self.device_info()
    }

    /// The device's music as albums. Reads the device on first call after connecting.
    pub fn device_collection(&self) -> Result<Vec<AlbumDto>> {
        let mut dev = self.device.lock().unwrap();
        if dev.library.is_none() {
            let storage = dev.storage;
            let Some(z) = dev.zune.as_mut() else { return Ok(Vec::new()) };
            let tracks = z.music_tracks().map_err(err)?;
            let albums = z.album_objects(storage).map_err(err)?;
            self.set_device_library(&mut dev, Some(DeviceLibrary { tracks, albums }));
        }
        let lib = dev.library.as_ref().unwrap();
        let by_handle: HashMap<u32, &DeviceAlbum> = lib.albums.iter().map(|a| (a.handle, a)).collect();

        // Group by linked album object, falling back to the album text.
        let mut groups: HashMap<String, (Option<&DeviceAlbum>, Vec<&DeviceTrack>)> = HashMap::new();
        for t in &lib.tracks {
            let album = t.album_ref.and_then(|h| by_handle.get(&h).copied());
            let key = match album {
                Some(a) => format!("h{}", a.handle),
                None => format!("t{}\u{1f}{}", norm(&t.album_artist), norm(&t.album)),
            };
            let e = groups.entry(key).or_insert((album, Vec::new()));
            e.1.push(t);
        }
        let mut out: Vec<AlbumDto> = groups
            .into_iter()
            .map(|(key, (album, mut tracks))| {
                tracks.sort_by_key(|t| (t.track_no, t.title.clone()));
                let first = tracks[0];
                let title = album.map(|a| a.title.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| first.album.clone());
                let artist = album.map(|a| a.artist.clone()).filter(|s| !s.is_empty()).unwrap_or_else(|| first.album_artist.clone());
                AlbumDto {
                    id: format!("dev:{key}"),
                    hue: hue(&format!("{artist}{title}")),
                    art: album.filter(|a| a.has_art).map(|a| format!("dev/{}", a.handle)),
                    year: None,
                    genre: first.genre.clone(),
                    tracks: tracks
                        .iter()
                        .map(|t| TrackDto {
                            // Device tracks are not playable from the computer; negative ids keep them apart.
                            id: -(t.handle as i64),
                            path: None,
                            disc: if t.track_no >= 100 { (t.track_no / 100) as u32 } else { 1 },
                            n: if t.track_no >= 100 { (t.track_no % 100) as u32 } else { t.track_no as u32 },
                            title: t.title.clone(),
                            artist: t.artist.clone(),
                            duration_ms: t.duration_ms as u64,
                            size: t.size,
                            codec: match t.format {
                                squirt_mtp::device::format::MP3 => "mp3",
                                squirt_mtp::device::format::WMA => "wma",
                                _ => "aac",
                            }
                            .into(),
                            on_device: true,
                            date: None,
                        })
                        .collect(),
                    title,
                    artist,
                }
            })
            .collect();
        out.sort_by_key(|a| (norm(&a.artist), norm(&a.title)));
        Ok(out)
    }

    // ---------------------------------------------------------- player

    fn with_player<T>(&self, f: impl FnOnce(&mut Playback) -> T) -> Result<T> {
        let mut slot = self.player.lock().unwrap();
        if slot.is_none() {
            let p = Player::new().map_err(err)?;
            p.set_volume(0.7);
            *slot = Some(Playback { player: p, queue: Vec::new() });
        }
        Ok(f(slot.as_mut().unwrap()))
    }

    /// Current player state; also records plays for finished tracks.
    /// Visualizer levels: `bands` values from 0 to 1, low to high frequency.
    pub fn player_spectrum(&self, bands: usize) -> Vec<f32> {
        let bands = bands.clamp(1, 256);
        match self.player.lock().unwrap().as_ref() {
            Some(pb) => pb.player.spectrum(bands),
            None => vec![0.0; bands],
        }
    }

    pub fn player_state(&self) -> PlayerStateDto {
        let slot = self.player.lock().unwrap();
        let Some(pb) = slot.as_ref() else {
            return PlayerStateDto { status: "stopped", track_id: None, position_ms: 0, duration_ms: 0, volume: 0.7, queue: Vec::new() };
        };
        for e in pb.player.take_events() {
            if let Event::TrackCompleted(i) = e {
                if let Some(id) = pb.queue.get(i) {
                    let _ = self.lib.lock().unwrap().record_play(*id);
                }
            }
        }
        let s = pb.player.state();
        PlayerStateDto {
            status: match s.status {
                Status::Playing => "playing",
                Status::Paused => "paused",
                Status::Stopped => "stopped",
            },
            track_id: s.index.and_then(|i| pb.queue.get(i).copied()),
            position_ms: s.position_ms,
            duration_ms: s.duration_ms,
            volume: s.volume,
            queue: pb.queue.clone(),
        }
    }

    /// Plays library tracks `ids`, starting at index `start`.
    pub fn play(&self, ids: Vec<i64>, start: usize) -> Result<PlayerStateDto> {
        let paths: Vec<PathBuf> = {
            let lib = self.lib.lock().unwrap();
            ids.iter().filter_map(|id| lib.track(*id).ok().flatten().map(|t| t.path)).collect()
        };
        if paths.len() != ids.len() || ids.is_empty() {
            return Err("some tracks are not in the library".into());
        }
        self.with_player(|pb| {
            pb.queue = ids;
            pb.player.play_queue(paths, start.min(pb.queue.len() - 1));
        })?;
        Ok(self.settled())
    }

    /// Title, artist, album and art for a library track.
    pub fn now_playing(&self, id: i64) -> Option<NowPlaying> {
        let lib = self.lib.lock().unwrap();
        let t = lib.track(id).ok().flatten()?;
        let art_path = t.art.as_deref().map(|k| lib.art_path(k)).filter(|p| p.exists());
        Some(NowPlaying {
            title: t.title,
            artist: t.artist,
            album: t.album,
            album_artist: t.album_artist,
            duration_ms: t.duration_ms,
            art_path,
        })
    }

    pub fn stop(&self) -> Result<PlayerStateDto> {
        self.with_player(|pb| pb.player.stop())?;
        Ok(self.settled())
    }

    pub fn toggle(&self) -> Result<PlayerStateDto> {
        self.with_player(|pb| pb.player.toggle())?;
        Ok(self.settled())
    }
    pub fn next(&self) -> Result<PlayerStateDto> {
        self.with_player(|pb| pb.player.next())?;
        Ok(self.settled())
    }
    pub fn prev(&self) -> Result<PlayerStateDto> {
        self.with_player(|pb| pb.player.prev())?;
        Ok(self.settled())
    }
    pub fn seek(&self, ms: u64) -> Result<PlayerStateDto> {
        self.with_player(|pb| pb.player.seek(ms))?;
        Ok(self.settled())
    }
    pub fn volume(&self, v: f32) -> Result<PlayerStateDto> {
        self.with_player(|pb| pb.player.set_volume(v))?;
        Ok(self.player_state())
    }

    /// State after giving the player thread a moment to act on a command.
    fn settled(&self) -> PlayerStateDto {
        std::thread::sleep(Duration::from_millis(30));
        self.player_state()
    }
}

/// The device's color, read from its own icon (MTP property 0xD405, a
/// Windows .ico). The Zune HD's icon shows the device itself, so the chin's
/// brightness tells slate (dark) from platinum (light). Verified on a slate
/// Zune HD, whose icon is slate; that platinum units ship a platinum icon is
/// assumed.
fn device_finish(z: &mut Zune) -> Option<String> {
    let d = z.raw(0x1015, &[0xD405]).ok()?;
    let n = u32::from_le_bytes(d.get(..4)?.try_into().ok()?) as usize;
    let ico = d.get(4..4 + n)?;
    let img = image::load_from_memory_with_format(ico, image::ImageFormat::Ico).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    // The chin: a band near the bottom of the device, around the centre.
    let (x0, x1) = (w * 36 / 100, w * 60 / 100);
    let (y0, y1) = (h * 80 / 100, h * 86 / 100);
    let (mut sum, mut count) = (0f64, 0u32);
    for y in y0..y1 {
        for x in x0..x1 {
            let p = img.get_pixel(x, y);
            if p[3] > 200 {
                sum += 0.2126 * p[0] as f64 + 0.7152 * p[1] as f64 + 0.0722 * p[2] as f64;
                count += 1;
            }
        }
    }
    if count < 20 {
        return None;
    }
    Some(if sum / count as f64 > 110.0 { "platinum" } else { "slate" }.into())
}

/// Which illustration fits: the Zune HD has its own USB id; the classic
/// models share one, so tell them apart by capacity (30 GB hard drive;
/// 4, 8 or 16 GB flash; 80 or 120 GB hard drive).
fn device_kind(model: squirt_mtp::devices::ZuneModel, capacity: u64) -> &'static str {
    use squirt_mtp::devices::ZuneModel;
    const GB: u64 = 1_000_000_000;
    match model {
        ZuneModel::Hd => "zunehd",
        ZuneModel::Classic if capacity > 50 * GB => "zunehdd",
        ZuneModel::Classic if capacity > 20 * GB => "zune30",
        ZuneModel::Classic => "zuneflash",
    }
}

/// Expands a leading `~` to the home directory.
fn expand_home(p: &str) -> PathBuf {
    match p.strip_prefix("~") {
        Some(rest) => std::env::var_os("HOME")
            .map(|h| PathBuf::from(h).join(rest.trim_start_matches(['/', '\\'])))
            .unwrap_or_else(|| PathBuf::from(p)),
        None => PathBuf::from(p),
    }
}

fn problem(dev: &mut DeviceSlot, msg: String) -> DeviceStatus {
    dev.status = DeviceStatus { problem: Some(msg), ..Default::default() };
    dev.status.clone()
}
