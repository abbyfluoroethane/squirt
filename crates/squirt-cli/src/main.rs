use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use squirt_mtp::{container::op, dataset::DeviceInfo, device::{self, format, prop, Zune}, library::{PushOutcome, Track}, devices, mtpz::{self, MtpzKeys}, transport::Transport};
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
    /// Dump every property of an object.
    Props {
        handles: Vec<String>,
        #[arg(long)]
        keys: Option<PathBuf>,
    },
    /// Show the metadata Squirt would send for a file (no device needed).
    Tags { file: PathBuf },
    /// Save an album's cover art to a file.
    Art {
        album: String,
        out: PathBuf,
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
            let meta = TrackMeta::read(&file)?;
            let art = meta.art.as_deref().map(scale_art).transpose()?;
            let bytes = std::fs::read(&file)?;
            let keys = load_keys(keys)?;
            let mut z = Zune::connect(Some(&keys))?;
            let storage = *z.storage_ids()?.first().context("no storage")?;
            let t0 = std::time::Instant::now();
            match z.push_track(storage, &meta.track, &bytes, art.as_deref())? {
                PushOutcome::AlreadyPresent { handle } => println!("already on device ({handle:#010x})"),
                PushOutcome::Pushed { handle, album, art_set } => {
                    let secs = t0.elapsed().as_secs_f64();
                    println!(
                        "pushed {} - {} -> {handle:#010x}, album {album:#010x}{} ({:.1} MB in {secs:.1}s)",
                        meta.track.artist,
                        meta.track.title,
                        if art_set { ", art set" } else { "" },
                        bytes.len() as f64 / 1e6
                    );
                }
            }
        }
        Cmd::Props { handles, keys } => {
            let keys = load_keys(keys)?;
            let mut z = Zune::connect(Some(&keys))?;
            for h in handles {
                let h = parse_handle(&h)?;
                println!("== {h:#010x}");
                for (code, dt, v) in z.all_props(h)? {
                    println!("  {code:#06x} dt {dt:#06x}  {v}");
                }
                if let Ok(r) = z.references(h) {
                    let refs: Vec<String> = r.iter().map(|x| format!("{x:#010x}")).collect();
                    println!("  references: [{}]", refs.join(", "));
                }
            }
        }
        Cmd::Tags { file } => {
            let m = TrackMeta::read(&file)?;
            println!("{:#?}", m.track);
            match &m.art {
                Some(a) => println!("cover art: {} bytes embedded, {} bytes after scaling", a.len(), scale_art(a)?.len()),
                None => println!("cover art: none"),
            }
            use lofty::prelude::*;
            let tagged = lofty::read_from_path(&file)?;
            for tag in tagged.tags() {
                println!("{:?}:", tag.tag_type());
                for item in tag.items() {
                    println!("  {:?} = {:?}", item.key(), item.value());
                }
            }
        }
        Cmd::Art { album, out, keys } => {
            let keys = load_keys(keys)?;
            let mut z = Zune::connect(Some(&keys))?;
            let bytes = z.prop_bytes(parse_handle(&album)?, 0xDC86)?;
            std::fs::write(&out, &bytes)?;
            println!("wrote {} bytes to {}", bytes.len(), out.display());
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
    track: Track,
    /// Embedded cover art, as stored in the file.
    art: Option<Vec<u8>>,
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
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let format = match ext.as_str() {
            "mp3" => format::MP3,
            "wma" => format::WMA,
            other => anyhow::bail!("unsupported file type: .{other}"),
        };
        let art = tag.and_then(|t| {
            use lofty::picture::PictureType;
            let pics = t.pictures();
            pics.iter()
                .find(|p| p.pic_type() == PictureType::CoverFront)
                .or_else(|| pics.first())
                .map(|p| p.data().to_vec())
        });
        Ok(Self {
            track: Track {
                title: get(&|t| t.title().map(|c| c.into_owned())).unwrap_or(stem),
                album: get(&|t| t.album().map(|c| c.into_owned())).unwrap_or_else(|| "Unknown Album".into()),
                album_artist: get(&|t| t.get_string(ItemKey::AlbumArtist).map(str::to_owned))
                    .unwrap_or_else(|| artist.clone()),
                artist,
                genre: get(&|t| t.genre().map(|c| c.into_owned())),
                track_number: tag.and_then(|t| t.track()).map(|n| n as u16),
                year: tag.and_then(|t| t.date()).map(|d| d.year as u32),
                duration_ms,
                format,
                extension: ext,
            },
            art,
        })
    }
}

/// Scales cover art to fit within 240x240 and re-encodes it as baseline JPEG,
/// which is what the Zune software stores.
fn scale_art(data: &[u8]) -> Result<Vec<u8>> {
    let img = image::load_from_memory(data).context("decoding cover art")?;
    let img = img.resize(240, 240, image::imageops::FilterType::Lanczos3).to_rgb8();
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88).encode_image(&img)?;
    Ok(out)
}
