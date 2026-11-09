//! Music library layout, matching what the Zune software writes:
//!
//! - Track files at `Music/<Album Artist>/<Album>/NN Title.ext`.
//! - Artist objects (format 0xB218) at `Artists/<Artist>.art`.
//! - Album objects (format 0xBA03) at `Albums/<Album Artist>--<Album>.alb`, linked
//!   to their artist (0xDAB9), holding cover art (0xDC86, JPEG within 240x240)
//!   and referencing their tracks.
//! - Tracks link to their album artist (0xDAB9) and album (0xDABB).
//!
//! If a track has no links, the device files it under "Unknown Artist" and
//! "Unknown Album", whatever its Artist/Album properties say.

use crate::container::op;
use crate::device::{encode_prop_list, format, prop, AnyValue, DeviceError, PropValue, Zune, ROOT};
use std::collections::HashMap;

pub mod zprop {
    /// Microsoft vendor property: handle of the linked artist object.
    pub const ARTIST_REF: u16 = 0xDAB9;
    /// Microsoft vendor property: handle of the linked album object.
    pub const ALBUM_REF: u16 = 0xDABB;
}

pub const FORMAT_ARTIST: u16 = 0xB218;
pub const FORMAT_JPEG: u16 = 0x3801;
pub const REPRESENTATIVE_SAMPLE_FORMAT: u16 = 0xDC81;
pub const REPRESENTATIVE_SAMPLE_DATA: u16 = 0xDC86;
pub const DATE_AUTHORED: u16 = 0xDC47;

#[derive(Debug, Clone)]
pub struct Track {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: Option<String>,
    pub track_number: Option<u16>,
    pub disc_number: Option<u16>,
    /// Number of discs in the album, when known.
    pub disc_total: Option<u16>,
    pub year: Option<u32>,
    pub duration_ms: u32,
    /// MTP object format, for example [`format::MP3`].
    pub format: u16,
    /// File extension without the dot, for example "mp3".
    pub extension: String,
}

#[derive(Debug)]
pub enum PushOutcome {
    Pushed { handle: u32, album: u32, art_set: bool },
    AlreadyPresent { handle: u32 },
}

/// Makes a string safe as a device file name.
pub fn safe_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.').to_string();
    if trimmed.is_empty() {
        "_".into()
    } else {
        trimmed
    }
}

/// The track number sent to the device. The Zune HD's disc number property
/// only accepts 0 or 1, so on multi-disc albums the disc goes into the track
/// number instead: disc 2, track 3 becomes 203. Tracks then sort correctly.
pub fn device_track_number(t: &Track) -> Option<u16> {
    let multi_disc = t.disc_total.is_some_and(|n| n > 1) || t.disc_number.is_some_and(|d| d > 1);
    match (t.track_number, t.disc_number) {
        (Some(n), Some(d)) if multi_disc && n < 100 => Some(d * 100 + n),
        (n, _) => n,
    }
}

/// `NN Title.ext` (the device may rename the file to its own pattern).
pub fn track_file_name(t: &Track) -> String {
    match device_track_number(t) {
        Some(n) => format!("{n:02} {}.{}", safe_name(&t.title), t.extension),
        None => format!("{}.{}", safe_name(&t.title), t.extension),
    }
}

fn str_prop(code: u16, s: &str) -> (u16, PropValue) {
    (code, PropValue::Str(s.to_string()))
}

impl Zune {
    /// Creates an object that has no file data (folder, artist, album).
    fn create_abstract(
        &mut self,
        storage: u32,
        parent: u32,
        fmt: u16,
        props: &[(u16, PropValue)],
    ) -> Result<u32, DeviceError> {
        let reply = self.transport().transact(
            op::SEND_OBJECT_PROP_LIST,
            &[storage, parent, fmt as u32, 0, 0],
            Some(&encode_prop_list(0, props)),
        )?;
        let h = reply.params.get(2).copied().ok_or(DeviceError::Unexpected("no handle for new object"))?;
        self.note_created(parent, props, h);
        Ok(h)
    }

    fn ensure_artist(&mut self, storage: u32, artists_dir: u32, name: &str) -> Result<u32, DeviceError> {
        let file = format!("{}.art", safe_name(name));
        if let Some(h) = self.find_child(storage, artists_dir, &file)? {
            return Ok(h);
        }
        self.create_abstract(
            storage,
            artists_dir,
            FORMAT_ARTIST,
            &[str_prop(prop::OBJECT_FILE_NAME, &file), str_prop(prop::NAME, name)],
        )
    }

    fn ensure_album(
        &mut self,
        storage: u32,
        albums_dir: u32,
        t: &Track,
        artist_obj: u32,
    ) -> Result<u32, DeviceError> {
        let file = format!("{}--{}.alb", safe_name(&t.album_artist), safe_name(&t.album));
        if let Some(h) = self.find_child(storage, albums_dir, &file)? {
            return Ok(h);
        }
        let mut props = vec![
            str_prop(prop::OBJECT_FILE_NAME, &file),
            str_prop(prop::NAME, &t.album),
            str_prop(prop::ARTIST, &t.album_artist),
            (zprop::ARTIST_REF, PropValue::U32(artist_obj)),
        ];
        if let Some(y) = t.year {
            props.push(str_prop(DATE_AUTHORED, &format!("{y:04}0101T000000.0")));
        }
        self.create_abstract(storage, albums_dir, format::ABSTRACT_AUDIO_ALBUM, &props)
    }

    pub(crate) fn set_prop(&mut self, handle: u32, code: u16, data: &[u8]) -> Result<(), DeviceError> {
        self.transport().transact(op::SET_OBJECT_PROP_VALUE, &[handle, code as u32], Some(data))?;
        Ok(())
    }

    /// Sets an album's cover art. `jpeg` should be a baseline JPEG within 240x240.
    pub fn set_album_art(&mut self, album: u32, jpeg: &[u8]) -> Result<(), DeviceError> {
        let mut data = (jpeg.len() as u32).to_le_bytes().to_vec();
        data.extend_from_slice(jpeg);
        self.set_prop(album, REPRESENTATIVE_SAMPLE_DATA, &data)?;
        self.set_prop(album, REPRESENTATIVE_SAMPLE_FORMAT, &FORMAT_JPEG.to_le_bytes())?;
        Ok(())
    }

    fn add_reference(&mut self, album: u32, track: u32) -> Result<(), DeviceError> {
        let mut refs = self.references(album)?;
        if !refs.contains(&track) {
            refs.push(track);
        }
        let mut data = (refs.len() as u32).to_le_bytes().to_vec();
        refs.iter().for_each(|r| data.extend(r.to_le_bytes()));
        self.transport().transact(op::SET_OBJECT_REFERENCES, &[album], Some(&data))?;
        Ok(())
    }

    /// Finds an already-pushed track without creating anything. Returns its handle.
    pub fn locate_track(&mut self, storage: u32, t: &Track) -> Result<Option<u32>, DeviceError> {
        let Some(music) = self.find_child(storage, ROOT, "Music")? else { return Ok(None) };
        let Some(artist_dir) = self.find_child(storage, music, &safe_name(&t.album_artist))? else { return Ok(None) };
        let Some(album_dir) = self.find_child(storage, artist_dir, &safe_name(&t.album))? else { return Ok(None) };
        self.find_track(album_dir, t.format, &t.title, device_track_number(t).unwrap_or(0))
    }

    /// Pushes one track with its artist/album objects and optional cover art
    /// (`art_jpeg`, already scaled). Art is only set when the album has none.
    pub fn push_track(
        &mut self,
        storage: u32,
        t: &Track,
        bytes: &[u8],
        art_jpeg: Option<&[u8]>,
    ) -> Result<PushOutcome, DeviceError> {
        self.push_track_progress(storage, t, bytes, art_jpeg, &mut |_| {})
    }

    /// Like [`Self::push_track`], reporting file bytes sent through `progress`.
    pub fn push_track_progress(
        &mut self,
        storage: u32,
        t: &Track,
        bytes: &[u8],
        art_jpeg: Option<&[u8]>,
        progress: &mut dyn FnMut(usize),
    ) -> Result<PushOutcome, DeviceError> {
        let music = self.ensure_folder(storage, ROOT, "Music")?;
        let artist_dir = self.ensure_folder(storage, music, &safe_name(&t.album_artist))?;
        let album_dir = self.ensure_folder(storage, artist_dir, &safe_name(&t.album))?;
        let filename = track_file_name(t);
        let track_no = device_track_number(t);
        if let Some(handle) = self.find_track(album_dir, t.format, &t.title, track_no.unwrap_or(0))? {
            return Ok(PushOutcome::AlreadyPresent { handle });
        }

        let artists = self.ensure_folder(storage, ROOT, "Artists")?;
        let albums = self.ensure_folder(storage, ROOT, "Albums")?;
        // Tracks link to the album artist, not their own artist, so featured
        // artists ("A & B", "A feat. C") do not each add an entry to the
        // device's Artists list. The track's own artist stays in its Artist property.
        let album_artist_obj = self.ensure_artist(storage, artists, &t.album_artist)?;
        let album = self.ensure_album(storage, albums, t, album_artist_obj)?;

        let mut props = vec![
            str_prop(prop::OBJECT_FILE_NAME, &filename),
            str_prop(prop::NAME, &t.title),
            str_prop(prop::ARTIST, &t.artist),
            str_prop(prop::ALBUM_NAME, &t.album),
            str_prop(prop::ALBUM_ARTIST, &t.album_artist),
            (prop::DURATION, PropValue::U32(t.duration_ms)),
            (zprop::ARTIST_REF, PropValue::U32(album_artist_obj)),
            (zprop::ALBUM_REF, PropValue::U32(album)),
        ];
        if let Some(n) = track_no {
            props.push((prop::TRACK, PropValue::U16(n)));
        }
        if let Some(g) = &t.genre {
            props.push(str_prop(prop::GENRE, g));
        }
        let handle = self.send_object_progress(storage, album_dir, t.format, &props, bytes, progress)?;
        self.note_track(album_dir, t.format, &t.title, track_no.unwrap_or(0), handle);
        self.add_reference(album, handle)?;

        let mut art_set = false;
        if let Some(jpeg) = art_jpeg {
            let has_art = self.prop_bytes(album, REPRESENTATIVE_SAMPLE_DATA).map(|b| !b.is_empty()).unwrap_or(false);
            if !has_art {
                self.set_album_art(album, jpeg)?;
                art_set = true;
            }
        }
        Ok(PushOutcome::Pushed { handle, album, art_set })
    }

    /// Deletes an album's tracks, then any folders they leave empty. The
    /// device removes the album and artist objects itself once nothing links
    /// to them. Returns the number of tracks deleted.
    pub fn delete_album(&mut self, storage: u32, album: u32) -> Result<usize, DeviceError> {
        let tracks = self.references(album)?;
        let mut folders = Vec::new();
        for &t in &tracks {
            let parent = self.prop_u32(t, prop::PARENT_OBJECT)?;
            self.delete_object(t)?;
            if !folders.contains(&parent) {
                folders.push(parent);
            }
        }
        // Remove emptied album folders, then emptied artist folders above them.
        for _ in 0..2 {
            let mut next = Vec::new();
            for f in folders {
                if f == ROOT || f == 0 || !self.object_handles(storage, 0, f)?.is_empty() {
                    continue;
                }
                let parent = self.prop_u32(f, prop::PARENT_OBJECT)?;
                let name = self.prop_string(parent, prop::OBJECT_FILE_NAME).unwrap_or_default();
                self.delete_object(f)?;
                // Never remove the top-level Music folder.
                if name != "Music" && !next.contains(&parent) {
                    next.push(parent);
                }
            }
            folders = next;
        }
        // If the device kept the album object, remove it too.
        if self.prop_u32(album, prop::OBJECT_FORMAT as u16).is_ok() {
            let _ = self.delete_object(album);
        }
        Ok(tracks.len())
    }
}

/// One change made (or planned) by [`Zune::merge_artist`].
#[derive(Debug, Clone)]
pub struct MergeChange {
    pub handle: u32,
    pub kind: &'static str,
    pub name: String,
    pub old_album_artist: String,
}

impl Zune {
    /// Finds an artist object in `Artists/` by display name.
    pub fn find_artist(&mut self, storage: u32, name: &str) -> Result<Option<u32>, DeviceError> {
        let artists = match self.find_child(storage, ROOT, "Artists")? {
            Some(a) => a,
            None => return Ok(None),
        };
        for h in self.object_handles(storage, FORMAT_ARTIST, artists)? {
            if self.prop_string(h, prop::NAME)? == name {
                return Ok(Some(h));
            }
        }
        Ok(None)
    }

    /// Relinks every album and track that points at artist `from` to artist
    /// `into`, sets their album-artist text to `into_name`, then deletes `from`.
    /// With `dry_run`, only reports what would change.
    pub fn merge_artist(
        &mut self,
        storage: u32,
        from: u32,
        into: u32,
        into_name: &str,
        dry_run: bool,
    ) -> Result<Vec<MergeChange>, DeviceError> {
        let mut changes = Vec::new();
        let albums = self.find_child(storage, ROOT, "Albums")?;
        let mut candidates: Vec<(u32, &'static str)> = Vec::new();
        if let Some(albums) = albums {
            for h in self.object_handles(storage, format::ABSTRACT_AUDIO_ALBUM, albums)? {
                candidates.push((h, "album"));
            }
        }
        for fmt in [format::MP3, format::WMA, format::M4A] {
            for h in self.object_handles(storage, fmt, 0)? {
                candidates.push((h, "track"));
            }
        }
        // Snapshot the links first: relinking an album can change its tracks.
        let mut linked = Vec::new();
        for (h, kind) in candidates {
            if self.prop_u32(h, zprop::ARTIST_REF).ok() == Some(from) {
                linked.push((h, kind));
            }
        }
        for (h, kind) in linked {
            let name = self.prop_string(h, prop::NAME).unwrap_or_default();
            let artist_prop = if kind == "album" { prop::ARTIST } else { prop::ALBUM_ARTIST };
            let old_album_artist = self.prop_string(h, artist_prop).unwrap_or_default();
            if !dry_run {
                // Relinking an album can make the device update its tracks
                // itself, after which writing them is refused (0x200F). So
                // accept a refused write when the value already matches.
                if let Err(e) = self.set_prop(h, zprop::ARTIST_REF, &into.to_le_bytes()) {
                    if self.prop_u32(h, zprop::ARTIST_REF).ok() != Some(into) {
                        return Err(e);
                    }
                }
                if let Err(e) = self.set_prop(h, artist_prop, &crate::dataset::encode_string(into_name)) {
                    if self.prop_string(h, artist_prop).ok().as_deref() != Some(into_name) {
                        return Err(e);
                    }
                }
            }
            changes.push(MergeChange { handle: h, kind, name, old_album_artist });
        }
        if !dry_run {
            // The device may already have removed the orphaned artist.
            let _ = self.delete_object(from);
        }
        Ok(changes)
    }
}

/// A track as the device knows it.
#[derive(Debug, Clone)]
pub struct DeviceTrack {
    pub handle: u32,
    pub format: u16,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: Option<String>,
    pub track_no: u16,
    pub duration_ms: u32,
    pub size: u64,
    /// Handle of the album object this track links to (0xDABB), if any.
    pub album_ref: Option<u32>,
    pub play_count: u32,
}

/// An album object (`Albums/*.alb`).
#[derive(Debug, Clone)]
pub struct DeviceAlbum {
    pub handle: u32,
    pub title: String,
    pub artist: String,
    pub has_art: bool,
}

impl Zune {
    /// Every property of every object of `format`, in one GetObjectPropList.
    /// About 15 ms per track on a Zune HD.
    fn props_by_format(&mut self, format: u16) -> Result<HashMap<u32, HashMap<u16, AnyValue>>, DeviceError> {
        self.bulk_props(0xFFFF_FFFF, format, 0)
    }

    /// The music on the device: every MP3, WMA and M4A track with its metadata.
    pub fn music_tracks(&mut self) -> Result<Vec<DeviceTrack>, DeviceError> {
        let mut out = Vec::new();
        for fmt in [format::MP3, format::WMA, format::M4A] {
            for (handle, p) in self.props_by_format(fmt)? {
                let s = |c: u16| match p.get(&c) {
                    Some(AnyValue::Str(v)) => v.clone(),
                    _ => String::new(),
                };
                let n = |c: u16| match p.get(&c) {
                    Some(AnyValue::Int(v)) => Some(*v),
                    _ => None,
                };
                let genre = s(prop::GENRE);
                out.push(DeviceTrack {
                    handle,
                    format: fmt,
                    title: s(prop::NAME),
                    artist: s(prop::ARTIST),
                    album: s(prop::ALBUM_NAME),
                    album_artist: s(prop::ALBUM_ARTIST),
                    genre: (!genre.is_empty()).then_some(genre),
                    track_no: n(prop::TRACK).unwrap_or(0) as u16,
                    duration_ms: n(prop::DURATION).unwrap_or(0) as u32,
                    size: n(prop::OBJECT_SIZE).unwrap_or(0) as u64,
                    album_ref: n(zprop::ALBUM_REF).map(|v| v as u32).filter(|&v| v != 0),
                    play_count: n(USE_COUNT).unwrap_or(0) as u32,
                });
            }
        }
        Ok(out)
    }

    /// The album objects on the device.
    pub fn album_objects(&mut self, storage: u32) -> Result<Vec<DeviceAlbum>, DeviceError> {
        let Some(albums) = self.find_child(storage, ROOT, "Albums")? else { return Ok(Vec::new()) };
        let mut out = Vec::new();
        for (handle, p) in self.bulk_props(albums, 0, 1)? {
            if handle == albums || !matches!(p.get(&prop::OBJECT_FORMAT), Some(AnyValue::Int(f)) if *f as u16 == format::ABSTRACT_AUDIO_ALBUM) {
                continue;
            }
            let s = |c: u16| match p.get(&c) {
                Some(AnyValue::Str(v)) => v.clone(),
                _ => String::new(),
            };
            let has_art = matches!(p.get(&REPRESENTATIVE_SAMPLE_DATA), Some(AnyValue::Array(a)) if !a.is_empty());
            out.push(DeviceAlbum { handle, title: s(prop::NAME), artist: s(prop::ARTIST), has_art });
        }
        Ok(out)
    }
}

/// Number of times a track has been played on the device.
pub const USE_COUNT: u16 = 0xDC91;

#[cfg(test)]
mod tests {
    use super::*;

    fn t(track: Option<u16>, disc: Option<u16>, total: Option<u16>) -> Track {
        Track {
            title: "A/B: C?".into(),
            artist: String::new(),
            album: String::new(),
            album_artist: String::new(),
            genre: None,
            track_number: track,
            disc_number: disc,
            disc_total: total,
            year: None,
            duration_ms: 0,
            format: format::M4A,
            extension: "m4a".into(),
        }
    }

    #[test]
    fn file_names() {
        assert_eq!(track_file_name(&t(Some(5), Some(1), Some(1))), "05 A_B_ C_.m4a");
        assert_eq!(track_file_name(&t(Some(5), Some(1), Some(2))), "105 A_B_ C_.m4a");
        assert_eq!(track_file_name(&t(Some(5), Some(2), None)), "205 A_B_ C_.m4a");
        assert_eq!(track_file_name(&t(None, Some(2), Some(2))), "A_B_ C_.m4a");
        assert_eq!(device_track_number(&t(Some(7), None, None)), Some(7));
    }
}
