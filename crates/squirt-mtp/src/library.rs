//! Music library layout, matching what the Zune software writes:
//!
//! - Track files at `Music/<Album Artist>/<Album>/NN Title.ext`.
//! - Artist objects (format 0xB218) at `Artists/<Artist>.art`.
//! - Album objects (format 0xBA03) at `Albums/<Album Artist>--<Album>.alb`, linked
//!   to their artist (0xDAB9), holding cover art (0xDC86, JPEG within 240x240)
//!   and referencing their tracks.
//! - Tracks link to their artist (0xDAB9) and album (0xDABB).
//!
//! If a track has no links, the device files it under "Unknown Artist" and
//! "Unknown Album", whatever its Artist/Album properties say.

use crate::container::op;
use crate::device::{encode_prop_list, format, prop, DeviceError, PropValue, Zune, ROOT};

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

    fn set_prop(&mut self, handle: u32, code: u16, data: &[u8]) -> Result<(), DeviceError> {
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
        let album_artist_obj = self.ensure_artist(storage, artists, &t.album_artist)?;
        let track_artist_obj = if t.artist == t.album_artist {
            album_artist_obj
        } else {
            self.ensure_artist(storage, artists, &t.artist)?
        };
        let album = self.ensure_album(storage, albums, t, album_artist_obj)?;

        let mut props = vec![
            str_prop(prop::OBJECT_FILE_NAME, &filename),
            str_prop(prop::NAME, &t.title),
            str_prop(prop::ARTIST, &t.artist),
            str_prop(prop::ALBUM_NAME, &t.album),
            str_prop(prop::ALBUM_ARTIST, &t.album_artist),
            (prop::DURATION, PropValue::U32(t.duration_ms)),
            (zprop::ARTIST_REF, PropValue::U32(track_artist_obj)),
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
