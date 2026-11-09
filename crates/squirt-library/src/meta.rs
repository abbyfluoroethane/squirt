//! Reading a track's tags, audio properties and cover art.

use lofty::file::FileType;
use lofty::prelude::*;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Mp3,
    Aac,
    Alac,
    Flac,
    Vorbis,
    Opus,
    Wav,
    Aiff,
    Wma,
    Other,
}

impl Codec {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mp3 => "mp3",
            Self::Aac => "aac",
            Self::Alac => "alac",
            Self::Flac => "flac",
            Self::Vorbis => "vorbis",
            Self::Opus => "opus",
            Self::Wav => "wav",
            Self::Aiff => "aiff",
            Self::Wma => "wma",
            Self::Other => "other",
        }
    }

    /// Whether a Zune HD can play this codec as-is.
    pub fn zune_native(self) -> bool {
        matches!(self, Self::Mp3 | Self::Aac | Self::Wma)
    }
}

#[derive(Debug, Clone)]
pub struct TrackMeta {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub genre: Option<String>,
    pub year: Option<u32>,
    pub track_no: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_no: Option<u32>,
    pub disc_total: Option<u32>,
    pub duration_ms: u64,
    pub codec: Codec,
    pub bitrate_kbps: Option<u32>,
    /// Embedded cover art (front cover preferred), or cover.jpg/folder.jpg next to the file.
    pub art: Option<Vec<u8>>,
}

fn codec_of(path: &Path, ft: FileType) -> Codec {
    match ft {
        FileType::Mpeg => Codec::Mp3,
        FileType::Aac => Codec::Aac,
        FileType::Flac => Codec::Flac,
        FileType::Vorbis => Codec::Vorbis,
        FileType::Opus => Codec::Opus,
        FileType::Wav => Codec::Wav,
        FileType::Aiff => Codec::Aiff,
        FileType::Mp4 => {
            use lofty::mp4::{Mp4Codec, Mp4File};
            let codec = std::fs::File::open(path)
                .ok()
                .and_then(|mut f| Mp4File::read_from(&mut f, lofty::config::ParseOptions::new()).ok())
                .and_then(|m| m.properties().codec());
            match codec {
                Some(Mp4Codec::AAC) => Codec::Aac,
                Some(Mp4Codec::ALAC) => Codec::Alac,
                Some(Mp4Codec::FLAC) => Codec::Flac,
                Some(Mp4Codec::MP3) => Codec::Mp3,
                _ => Codec::Other,
            }
        }
        _ if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("wma")) => Codec::Wma,
        _ => Codec::Other,
    }
}

/// Cover art next to the file (cover.jpg, folder.png, ...).
fn folder_art(path: &Path) -> Option<Vec<u8>> {
    let dir = path.parent()?;
    let mut entries: Vec<_> = std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    entries
        .into_iter()
        .find(|p| {
            let stem = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
            let ext = p.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
            matches!(stem.as_str(), "cover" | "folder" | "front" | "album") && matches!(ext.as_str(), "jpg" | "jpeg" | "png")
        })
        .and_then(|p| std::fs::read(p).ok())
}

/// Reads one file's metadata. Missing tags fall back to the file name and
/// "Unknown Artist"/"Unknown Album"; the album artist falls back to the artist.
pub fn read_track(path: &Path) -> Result<TrackMeta, String> {
    let tagged = lofty::read_from_path(path).map_err(|e| e.to_string())?;
    let props = tagged.properties();
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let text = |f: &dyn Fn(&lofty::tag::Tag) -> Option<String>| tag.and_then(f).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());

    let artist = text(&|t| t.artist().map(|c| c.into_owned())).unwrap_or_else(|| "Unknown Artist".into());
    let art = tag
        .and_then(|t| {
            use lofty::picture::PictureType;
            let pics = t.pictures();
            pics.iter().find(|p| p.pic_type() == PictureType::CoverFront).or_else(|| pics.first()).map(|p| p.data().to_vec())
        })
        .or_else(|| folder_art(path));

    Ok(TrackMeta {
        title: text(&|t| t.title().map(|c| c.into_owned()))
            .unwrap_or_else(|| path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()),
        album: text(&|t| t.album().map(|c| c.into_owned())).unwrap_or_else(|| "Unknown Album".into()),
        album_artist: text(&|t| t.get_string(ItemKey::AlbumArtist).map(str::to_owned)).unwrap_or_else(|| artist.clone()),
        artist,
        genre: text(&|t| t.genre().map(|c| c.into_owned())),
        year: tag.and_then(|t| t.date()).map(|d| d.year as u32),
        track_no: tag.and_then(|t| t.track()),
        track_total: tag.and_then(|t| t.track_total()),
        disc_no: tag.and_then(|t| t.disk()),
        disc_total: tag.and_then(|t| t.disk_total()),
        duration_ms: props.duration().as_millis() as u64,
        codec: codec_of(path, tagged.file_type()),
        bitrate_kbps: props.audio_bitrate().or(props.overall_bitrate()),
        art,
    })
}
