//! Reading a file for the Zune: its tags as the device wants them, and its
//! cover art scaled the way the Zune software stored it. Shared by the
//! app's sync and the CLI's `push`.

use lofty::prelude::*;
use squirt_mtp::device::format;
use squirt_mtp::library::Track;
use std::path::Path;

/// A track's device metadata and its embedded (or folder) cover art.
pub struct PushMeta {
    pub track: Track,
    /// Cover art as stored in the file or folder, not yet scaled.
    pub art: Option<Vec<u8>>,
}

/// File types the Zune plays without transcoding.
pub const PUSH_EXTENSIONS: &[&str] = &["mp3", "wma", "m4a"];

/// Reads tags for pushing. Fails for formats the Zune can't play
/// (ALAC .m4a, FLAC, ...), with a message saying so.
pub fn read_meta(path: &Path) -> Result<PushMeta, String> {
    let tagged = lofty::read_from_path(path).map_err(|e| format!("reading tags from {}: {e}", path.display()))?;
    let duration_ms = tagged.properties().duration().as_millis() as u32;
    let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
    let get = |f: &dyn Fn(&lofty::tag::Tag) -> Option<String>| tag.and_then(f).filter(|s| !s.trim().is_empty());
    let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let artist = get(&|t| t.artist().map(|c| c.into_owned())).unwrap_or_else(|| "Unknown Artist".into());
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let format = match ext.as_str() {
        "mp3" => format::MP3,
        "wma" => format::WMA,
        "m4a" => {
            use lofty::mp4::{Mp4Codec, Mp4File};
            let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
            let mp4 = Mp4File::read_from(&mut f, lofty::config::ParseOptions::new()).map_err(|e| e.to_string())?;
            match mp4.properties().codec() {
                Some(Mp4Codec::AAC) => format::M4A,
                other => return Err(format!("the Zune plays AAC .m4a files only, this one is {other:?}")),
            }
        }
        other => return Err(format!("the Zune can't play .{other} files")),
    };
    let art = tag
        .and_then(|t| {
            use lofty::picture::PictureType;
            let pics = t.pictures();
            pics.iter().find(|p| p.pic_type() == PictureType::CoverFront).or_else(|| pics.first()).map(|p| p.data().to_vec())
        })
        .or_else(|| folder_art(path));
    Ok(PushMeta {
        track: Track {
            title: get(&|t| t.title().map(|c| c.into_owned())).unwrap_or(stem),
            album: get(&|t| t.album().map(|c| c.into_owned())).unwrap_or_else(|| "Unknown Album".into()),
            album_artist: get(&|t| t.get_string(ItemKey::AlbumArtist).map(str::to_owned)).unwrap_or_else(|| artist.clone()),
            artist,
            genre: get(&|t| t.genre().map(|c| c.into_owned())),
            track_number: tag.and_then(|t| t.track()).map(|n| n as u16),
            disc_number: tag.and_then(|t| t.disk()).map(|n| n as u16),
            disc_total: tag.and_then(|t| t.disk_total()).map(|n| n as u16),
            year: tag.and_then(|t| t.date()).map(|d| d.year as u32),
            duration_ms,
            format,
            extension: ext,
        },
        art,
    })
}

/// Scales cover art to fit within 240x240 and re-encodes it as baseline JPEG,
/// which is what the Zune software stores.
pub fn scale_art(data: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(data).map_err(|e| format!("decoding cover art: {e}"))?;
    let img = img.resize(240, 240, image::imageops::FilterType::Lanczos3).to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88).encode_image(&img).map_err(|e| e.to_string())?;
    Ok(out)
}

/// cover.jpg, folder.png and the like, next to the track.
fn folder_art(track: &Path) -> Option<Vec<u8>> {
    let dir = track.parent()?;
    let names = ["cover", "folder", "front", "album"];
    let mut entries: Vec<_> = std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    entries
        .into_iter()
        .find(|p| {
            let stem = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
            let ext = p.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
            names.contains(&stem.as_str()) && matches!(ext.as_str(), "jpg" | "jpeg" | "png")
        })
        .and_then(|p| std::fs::read(p).ok())
}
