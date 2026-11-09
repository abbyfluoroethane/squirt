use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use squirt_mtp::{container::op, dataset::DeviceInfo, device::{self, format, prop, PropValue, Zune}, devices, mtpz::{self, MtpzKeys}, transport::Transport};
use std::path::PathBuf;

/// Load music, video and more onto a Zune.
#[derive(Parser)]
#[command(name = "squirt", version)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List connected Zunes.
    Detect,
    /// Show device info and supported operations for the first Zune found.
    Info,
    /// Run the MTPZ handshake with the first Zune found.
    Auth {
        /// Key file (default: ~/.mtpz-data).
        #[arg(long)]
        keys: Option<PathBuf>,
    },
    /// List storage and the objects in a folder (default: root).
    Ls {
        /// Parent object handle to list (hex or decimal).
        #[arg(long)]
        parent: Option<String>,
        #[arg(long)]
        keys: Option<PathBuf>,
    },
    /// Copy an MP3 to Music/<Artist>/<Album>/, with metadata from its tags.
    Push {
        file: PathBuf,
        #[arg(long)]
        keys: Option<PathBuf>,
    },
    /// Delete an object by handle.
    Rm {
        handle: String,
        #[arg(long)]
        keys: Option<PathBuf>,
    },
    /// Check an MTPZ key file (default: ~/.mtpz-data).
    Keys {
        path: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    match Cli::parse().cmd {
        Cmd::Detect => {
            let zunes = devices::find_zunes().context("could not list USB devices")?;
            if zunes.is_empty() {
                println!("No Zune found. Plug one in and unlock it.");
            }
            for z in zunes {
                println!(
                    "{}  serial={}",
                    z.model.name(),
                    z.serial.as_deref().unwrap_or("?")
                );
            }
        }
        Cmd::Info => {
            let z = devices::find_zunes()?
                .into_iter()
                .next()
                .context("no Zune found")?;
            let mut t = Transport::open(&z.info).context("could not open device")?;
            let reply = t.transact(op::GET_DEVICE_INFO, &[], None)?;
            let info = DeviceInfo::parse(&reply.data.context("no DeviceInfo data")?)?;
            println!("{} {} (firmware {})", info.manufacturer, info.model, info.device_version);
            println!("vendor extension: {:#x} v{} \"{}\"", info.vendor_extension_id,
                info.vendor_extension_version, info.vendor_extension_desc);
            let mtpz_ops = [op::WMDRMPD_SEND_APP_REQUEST, op::WMDRMPD_GET_APP_RESPONSE,
                op::ENABLE_TRUSTED_FILES_OPERATIONS, op::END_TRUSTED_APP_SESSION];
            println!("MTPZ extension: {}, MTPZ ops: {}",
                mtpz::advertises_mtpz(&info.vendor_extension_desc),
                mtpz_ops.iter().all(|&o| info.supports(o)));
            let ops: Vec<String> = info.operations.iter().map(|o| format!("{o:#06x}")).collect();
            println!("operations ({}): {}", ops.len(), ops.join(" "));
            t.open_session(1).context("OpenSession failed")?;
            println!("OpenSession: OK");
        }
        Cmd::Auth { keys } => {
            let keys = load_keys(keys)?;
            let z = devices::find_zunes()?.into_iter().next().context("no Zune found")?;
            let mut t = Transport::open(&z.info).context("could not open device")?;
            let reply = t.transact(op::GET_DEVICE_INFO, &[], None)?;
            let info = DeviceInfo::parse(&reply.data.context("no DeviceInfo data")?)?;
            if !mtpz::advertises_mtpz(&info.vendor_extension_desc) {
                anyhow::bail!("{} does not advertise MTPZ", info.model);
            }
            t.open_session(1).context("OpenSession failed")?;
            // The reference flow lists storage before the handshake (spec 4, uncertainty 6).
            let ids = t.transact(op::GET_STORAGE_IDS, &[], None)?;
            println!("storage ids: {} bytes", ids.data.map_or(0, |d| d.len()));
            let m2 = mtpz::authenticate(&mut t, &keys).context("MTPZ handshake failed")?;
            if !m2.oaep_standard {
                eprintln!("note: device OAEP block is not standard empty-label layout (accepted anyway)");
            }
            println!("MTPZ handshake: OK. {} is trusted for transfers.", info.model);
        }
        Cmd::Ls { parent, keys } => {
            let keys = load_keys(keys)?;
            let mut z = Zune::connect(Some(&keys))?;
            let parent = match parent {
                None => device::ROOT,
                Some(p) => parse_handle(&p)?,
            };
            for id in z.storage_ids()? {
                let si = z.storage_info(id)?;
                println!(
                    "storage {id:#010x} \"{}\": {:.1} GB free of {:.1} GB",
                    si.description,
                    si.free_bytes as f64 / 1e9,
                    si.max_capacity as f64 / 1e9
                );
                for h in z.object_handles(id, 0, parent)? {
                    let name = z.prop_string(h, prop::OBJECT_FILE_NAME).unwrap_or_default();
                    let fmt = z.prop_u16(h, prop::OBJECT_FORMAT).unwrap_or(0);
                    println!("  {h:#010x}  fmt {fmt:#06x}  {name}");
                }
            }
            if parent == device::ROOT {
                let props: Vec<String> = z.props_supported(format::MP3)?.iter().map(|p| format!("{p:#06x}")).collect();
                println!("MP3 props supported: {}", props.join(" "));
            }
        }
        Cmd::Push { file, keys } => {
            let track = TrackMeta::read(&file)?;
            let bytes = std::fs::read(&file)?;
            let keys = load_keys(keys)?;
            let mut z = Zune::connect(Some(&keys))?;
            let storage = *z.storage_ids()?.first().context("no storage")?;
            let music = z.ensure_folder(storage, device::ROOT, "Music")?;
            let artist_dir = z.ensure_folder(storage, music, &safe_name(&track.album_artist))?;
            let album_dir = z.ensure_folder(storage, artist_dir, &safe_name(&track.album))?;
            let filename = match track.track {
                Some(n) => format!("{n:02} {}.mp3", safe_name(&track.title)),
                None => format!("{}.mp3", safe_name(&track.title)),
            };
            if let Some(h) = z.find_child(storage, album_dir, &filename)? {
                println!("already on device: {filename} ({h:#010x})");
                return Ok(());
            }
            let mut props = vec![
                (prop::OBJECT_FILE_NAME, PropValue::Str(filename.clone())),
                (prop::NAME, PropValue::Str(track.title.clone())),
                (prop::ARTIST, PropValue::Str(track.artist.clone())),
                (prop::ALBUM_NAME, PropValue::Str(track.album.clone())),
                (prop::ALBUM_ARTIST, PropValue::Str(track.album_artist.clone())),
                (prop::DURATION, PropValue::U32(track.duration_ms)),
            ];
            if let Some(n) = track.track {
                props.push((prop::TRACK, PropValue::U16(n as u16)));
            }
            if let Some(g) = &track.genre {
                props.push((prop::GENRE, PropValue::Str(g.clone())));
            }
            let t0 = std::time::Instant::now();
            let h = z.send_object(storage, album_dir, format::MP3, &props, &bytes)?;
            let secs = t0.elapsed().as_secs_f64();
            println!(
                "pushed {filename} -> {h:#010x} ({:.1} MB in {secs:.1}s, {:.1} MB/s)",
                bytes.len() as f64 / 1e6,
                bytes.len() as f64 / 1e6 / secs
            );
        }
        Cmd::Rm { handle, keys } => {
            let keys = load_keys(keys)?;
            let mut z = Zune::connect(Some(&keys))?;
            let h = parse_handle(&handle)?;
            let name = z.prop_string(h, prop::OBJECT_FILE_NAME).unwrap_or_default();
            z.delete_object(h)?;
            println!("deleted {h:#010x} {name}");
        }
        Cmd::Keys { path } => {
            let keys = load_keys(path)?;
            println!("OK: {keys:?}");
        }
    }
    Ok(())
}

fn load_keys(path: Option<PathBuf>) -> Result<MtpzKeys> {
    let path = path
        .or_else(MtpzKeys::default_path)
        .context("no key path given and no home directory found")?;
    MtpzKeys::load(&path).with_context(|| format!("MTPZ key file {}", path.display()))
}

fn parse_handle(s: &str) -> Result<u32> {
    Ok(match s.strip_prefix("0x") {
        Some(h) => u32::from_str_radix(h, 16)?,
        None => s.parse()?,
    })
}

struct TrackMeta {
    title: String,
    artist: String,
    album: String,
    album_artist: String,
    genre: Option<String>,
    track: Option<u32>,
    duration_ms: u32,
}

impl TrackMeta {
    fn read(path: &std::path::Path) -> Result<Self> {
        use lofty::prelude::*;
        let tagged = lofty::read_from_path(path).with_context(|| format!("reading tags from {}", path.display()))?;
        let duration_ms = tagged.properties().duration().as_millis() as u32;
        let tag = tagged.primary_tag().or_else(|| tagged.first_tag());
        let get = |f: &dyn Fn(&lofty::tag::Tag) -> Option<String>| tag.and_then(f).filter(|s| !s.trim().is_empty());
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let artist = get(&|t| t.artist().map(|c| c.into_owned())).unwrap_or_else(|| "Unknown Artist".into());
        Ok(Self {
            title: get(&|t| t.title().map(|c| c.into_owned())).unwrap_or(stem),
            album: get(&|t| t.album().map(|c| c.into_owned())).unwrap_or_else(|| "Unknown Album".into()),
            album_artist: get(&|t| t.get_string(ItemKey::AlbumArtist).map(str::to_owned)).unwrap_or_else(|| artist.clone()),
            artist,
            genre: get(&|t| t.genre().map(|c| c.into_owned())),
            track: tag.and_then(|t| t.track()),
            duration_ms,
        })
    }
}

/// Makes a string safe as a device file name.
fn safe_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { '_' } else { c })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.').to_string();
    if trimmed.is_empty() { "_".into() } else { trimmed }
}
