mod serve;

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
    /// Copy music files and folders to the Zune, with metadata and cover art.
    Push {
        /// Files or folders (folders are searched recursively).
        #[arg(required = true)]
        paths: Vec<PathBuf>,
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
    /// List albums, or delete albums whose name matches (with their tracks).
    Albums {
        /// Delete every album whose name contains this text.
        #[arg(long)]
        delete_matching: Option<String>,
        #[arg(long)]
        keys: Option<PathBuf>,
    },
    /// Move everything filed under artist FROM to artist INTO, then remove FROM.
    MergeArtist {
        from: String,
        into: String,
        /// Show what would change without changing anything.
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        keys: Option<PathBuf>,
    },
    /// Check that files are on the device with the right size.
    Verify {
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Delete tracks whose size on the device is wrong, so a push re-sends them.
        #[arg(long)]
        delete_bad: bool,
        #[arg(long)]
        keys: Option<PathBuf>,
    },
    /// Manage the local music library (watched folders).
    Library {
        #[command(subcommand)]
        cmd: LibCmd,
    },
    /// Play files, or an album from the library.
    Play {
        paths: Vec<PathBuf>,
        /// Play this library album (matched by name, case-insensitive).
        #[arg(long)]
        album: Option<String>,
        /// Volume from 0 to 1.
        #[arg(long, default_value_t = 0.7)]
        volume: f32,
        /// Stop after this many seconds.
        #[arg(long)]
        seconds: Option<f64>,
        /// Seek to this many seconds into the first track after 1 s (for testing).
        #[arg(long)]
        seek_test: Option<u64>,
    },
    /// Serve a local JSON API for developing the UI in a browser.
    Serve {
        #[arg(long, default_value_t = 5190)]
        port: u16,
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

#[derive(Subcommand)]
enum LibCmd {
    /// Watch a folder.
    Add { path: PathBuf },
    /// List watched folders.
    Folders,
    /// Scan watched folders for new, changed and removed files.
    Scan,
    /// List albums.
    Albums,
}

fn open_library() -> Result<squirt_library::Library> {
    let (db, art) = squirt_library::Library::default_dirs().context("no data directory")?;
    Ok(squirt_library::Library::open(&db, &art)?)
}

fn library(cmd: LibCmd) -> Result<()> {
    let mut lib = open_library()?;
    match cmd {
        LibCmd::Add { path } => {
            let f = lib.add_folder(&path, squirt_library::Kind::Music)?;
            println!("watching {}", f.path.display());
        }
        LibCmd::Folders => {
            for f in lib.folders()? {
                println!("{}  {}", f.id, f.path.display());
            }
        }
        LibCmd::Scan => {
            let t0 = std::time::Instant::now();
            let r = lib.scan(&mut |_| {})?;
            println!(
                "{} files: {} added, {} updated, {} removed, {} errors ({:.1}s)",
                r.seen, r.added, r.updated, r.removed, r.errors.len(), t0.elapsed().as_secs_f64()
            );
            for (p, e) in r.errors.iter().take(10) {
                println!("  error: {}: {e}", p.display());
            }
        }
        LibCmd::Albums => {
            for a in lib.albums(None)? {
                println!(
                    "{} - {} ({}) {} tracks, {} min{}",
                    a.artist,
                    a.title,
                    a.year.map(|y| y.to_string()).unwrap_or_default(),
                    a.track_count,
                    a.duration_ms / 60_000,
                    if a.art.is_some() { ", art" } else { "" }
                );
            }
        }
    }
    Ok(())
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
            let fmts: Vec<String> = info.playback_formats.iter().map(|o| format!("{o:#06x}")).collect();
            println!("playback formats ({}): {}", fmts.len(), fmts.join(" "));
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
        Cmd::Push { paths, keys } => push(paths, keys)?,
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
        Cmd::Albums { delete_matching, keys } => {
            let keys = load_keys(keys)?;
            let mut z = Zune::connect(Some(&keys))?;
            let storage = *z.storage_ids()?.first().context("no storage")?;
            let albums_dir = z.find_child(storage, device::ROOT, "Albums")?.context("no Albums folder")?;
            for h in z.object_handles(storage, 0, albums_dir)? {
                let name = z.prop_string(h, prop::NAME).unwrap_or_default();
                let artist = z.prop_string(h, prop::ARTIST).unwrap_or_default();
                match &delete_matching {
                    Some(m) if name.contains(m.as_str()) => {
                        let n = z.delete_album(storage, h)?;
                        println!("deleted {artist} - {name} ({n} tracks)");
                    }
                    Some(_) => {}
                    None => println!("{h:#010x}  {artist} - {name}"),
                }
            }
        }
        Cmd::MergeArtist { from, into, dry_run, keys } => {
            let keys = load_keys(keys)?;
            let mut z = Zune::connect(Some(&keys))?;
            let storage = *z.storage_ids()?.first().context("no storage")?;
            let from_h = z.find_artist(storage, &from)?.with_context(|| format!("no artist named {from:?}"))?;
            let into_h = z.find_artist(storage, &into)?.with_context(|| format!("no artist named {into:?}"))?;
            anyhow::ensure!(from_h != into_h, "FROM and INTO are the same artist");
            let changes = z.merge_artist(storage, from_h, into_h, &into, dry_run)?;
            let verb = if dry_run { "would relink" } else { "relinked" };
            for c in &changes {
                println!("{verb} {} {:#010x} {:?} (was album artist {:?}, link {from_h:#010x})", c.kind, c.handle, c.name, c.old_album_artist);
            }
            if dry_run {
                println!("dry run: {} objects, and {from:?} ({from_h:#010x}) would be removed", changes.len());
            } else {
                println!("merged {from:?} into {into:?}: {} objects relinked", changes.len());
            }
        }
        Cmd::Verify { paths, delete_bad, keys } => {
            let files = collect_audio(&paths)?;
            let keys = load_keys(keys)?;
            let mut z = Zune::connect(Some(&keys))?;
            let storage = *z.storage_ids()?.first().context("no storage")?;
            let (mut ok, mut missing, mut bad) = (0, 0, 0);
            for f in &files {
                let meta = TrackMeta::read(f)?;
                let local = f.metadata()?.len();
                match z.locate_track(storage, &meta.track)? {
                    None => {
                        missing += 1;
                        println!("missing: {}", f.display());
                    }
                    Some(h) => {
                        let remote = z.prop_u64(h, prop::OBJECT_SIZE)?;
                        if remote == local {
                            ok += 1;
                        } else {
                            bad += 1;
                            println!("size mismatch ({remote} on device, {local} local): {}", f.display());
                            if delete_bad {
                                z.delete_object(h)?;
                                println!("  deleted {h:#010x}");
                            }
                        }
                    }
                }
            }
            println!("{ok} ok, {missing} missing, {bad} wrong size");
        }
        Cmd::Library { cmd } => library(cmd)?,
        Cmd::Serve { port } => serve::serve(port)?,
        Cmd::Play { paths, album, volume, seconds, seek_test } => play(paths, album, volume, seconds, seek_test)?,
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
            "m4a" => {
                use lofty::mp4::{Mp4Codec, Mp4File};
                let mut f = std::fs::File::open(path)?;
                let mp4 = Mp4File::read_from(&mut f, lofty::config::ParseOptions::new())?;
                match mp4.properties().codec() {
                    Some(Mp4Codec::AAC) => format::M4A,
                    other => anyhow::bail!("the Zune plays AAC .m4a files only, this one is {other:?} (transcoding is not supported yet)"),
                }
            }
            other => anyhow::bail!("unsupported file type: .{other}"),
        };
        let art = tag.and_then(|t| {
            use lofty::picture::PictureType;
            let pics = t.pictures();
            pics.iter()
                .find(|p| p.pic_type() == PictureType::CoverFront)
                .or_else(|| pics.first())
                .map(|p| p.data().to_vec())
        })
        .or_else(|| folder_art(path));
        Ok(Self {
            track: Track {
                title: get(&|t| t.title().map(|c| c.into_owned())).unwrap_or(stem),
                album: get(&|t| t.album().map(|c| c.into_owned())).unwrap_or_else(|| "Unknown Album".into()),
                album_artist: get(&|t| t.get_string(ItemKey::AlbumArtist).map(str::to_owned))
                    .unwrap_or_else(|| artist.clone()),
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

const AUDIO_EXTENSIONS: &[&str] = &["mp3", "wma", "m4a"];

/// Expands files and folders into a sorted list of supported audio files.
fn collect_audio(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    fn walk(p: &std::path::Path, out: &mut Vec<PathBuf>) -> Result<()> {
        if p.is_dir() {
            let mut entries: Vec<_> = std::fs::read_dir(p)?.filter_map(|e| e.ok().map(|e| e.path())).collect();
            entries.sort();
            for e in entries {
                if !e.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')) {
                    walk(&e, out)?;
                }
            }
        } else if p
            .extension()
            .is_some_and(|e| AUDIO_EXTENSIONS.contains(&e.to_string_lossy().to_lowercase().as_str()))
        {
            out.push(p.to_path_buf());
        }
        Ok(())
    }
    let mut out = Vec::new();
    for p in paths {
        anyhow::ensure!(p.exists(), "{} does not exist", p.display());
        walk(p, &mut out)?;
    }
    Ok(out)
}

fn push(paths: Vec<PathBuf>, keys: Option<PathBuf>) -> Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};

    let files = collect_audio(&paths)?;
    anyhow::ensure!(!files.is_empty(), "no supported audio files found ({})", AUDIO_EXTENSIONS.join(", "));
    let total_bytes: u64 = files.iter().filter_map(|f| f.metadata().ok()).map(|m| m.len()).sum();

    let keys = load_keys(keys)?;
    let mut z = Zune::connect(Some(&keys))?;
    let storage = *z.storage_ids()?.first().context("no storage")?;
    let free = z.storage_info(storage)?.free_bytes;
    anyhow::ensure!(
        total_bytes < free,
        "not enough space: need {:.1} MB, {:.1} MB free",
        total_bytes as f64 / 1e6,
        free as f64 / 1e6
    );

    let bar = ProgressBar::new(total_bytes);
    bar.set_style(
        ProgressStyle::with_template("{bar:30.cyan/blue} {bytes}/{total_bytes} {binary_bytes_per_sec} eta {eta}  {msg}")
            .unwrap()
            .progress_chars("=> "),
    );
    let (mut pushed, mut skipped, mut failed) = (0, 0, Vec::new());
    let mut pushed_bytes = 0u64;
    let t0 = std::time::Instant::now();
    // Scaled art per album, so each album's art is decoded and scaled once.
    let mut art_cache: std::collections::HashMap<(String, String), Option<Vec<u8>>> = Default::default();

    for (i, file) in files.iter().enumerate() {
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        let base = bar.position();
        let mut attempt = 0;
        let result = loop {
            let result = (|| -> Result<PushOutcome> {
                let meta = TrackMeta::read(file)?;
                bar.set_message(format!("[{}/{}] {} - {}", i + 1, files.len(), meta.track.artist, meta.track.title));
                let key = (meta.track.album_artist.clone(), meta.track.album.clone());
                if !art_cache.contains_key(&key) {
                    let art = match meta.art.as_deref().map(scale_art) {
                        Some(Ok(a)) => Some(a),
                        Some(Err(e)) => {
                            eprintln!("warning: cover art for {:?} unusable: {e:#}", meta.track.album);
                            None
                        }
                        None => None,
                    };
                    art_cache.insert(key.clone(), art);
                }
                // After a reconnect, a partly written copy may be on the device.
                if attempt > 0 {
                    if let Some(h) = z.locate_track(storage, &meta.track)? {
                        if z.prop_u64(h, prop::OBJECT_SIZE)? != size {
                            z.delete_object(h)?;
                        }
                    }
                }
                let bytes = std::fs::read(file)?;
                Ok(z.push_track_progress(storage, &meta.track, &bytes, art_cache[&key].as_deref(), &mut |sent| {
                    bar.set_position(base + sent as u64)
                })?)
            })();
            match result {
                Err(e) if attempt < 3 && is_connection_error(&e) => {
                    attempt += 1;
                    eprintln!("connection lost ({e:#}); waiting for the Zune to come back...");
                    bar.set_position(base);
                    drop(z); // Release the USB interface before claiming it again.
                    z = reconnect(&keys)?;
                    eprintln!("reconnected; retrying {}", file.display());
                }
                other => break other,
            }
        };
        bar.set_position(base + size);
        match result {
            Ok(PushOutcome::Pushed { .. }) => {
                pushed += 1;
                pushed_bytes += size;
            }
            Ok(PushOutcome::AlreadyPresent { .. }) => skipped += 1,
            Err(e) => {
                eprintln!("failed: {}: {e:#}", file.display());
                failed.push(file.clone());
            }
        }
    }
    bar.finish_and_clear();
    let secs = t0.elapsed().as_secs_f64();
    println!(
        "pushed {pushed}, already on device {skipped}, failed {}. {:.1} MB in {secs:.1}s ({:.1} MB/s)",
        failed.len(),
        pushed_bytes as f64 / 1e6,
        pushed_bytes as f64 / 1e6 / secs.max(0.001)
    );
    anyhow::ensure!(failed.is_empty(), "{} file(s) failed", failed.len());
    Ok(())
}

/// Cover art next to the file (cover.jpg, folder.jpg, and similar), for files without embedded art.
fn folder_art(track: &std::path::Path) -> Option<Vec<u8>> {
    let dir = track.parent()?;
    let names = ["cover", "folder", "front", "album"];
    let mut entries: Vec<_> = std::fs::read_dir(dir).ok()?.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    entries.sort();
    entries.into_iter().find(|p| {
        let stem = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        let ext = p.extension().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        names.contains(&stem.as_str()) && matches!(ext.as_str(), "jpg" | "jpeg" | "png")
    }).and_then(|p| std::fs::read(p).ok())
}

/// True for USB-level failures (disconnect, timeout), not MTP error responses.
fn is_connection_error(e: &anyhow::Error) -> bool {
    use squirt_mtp::device::DeviceError;
    use squirt_mtp::transport::TransportError;
    matches!(
        e.downcast_ref::<DeviceError>(),
        Some(DeviceError::Transport(TransportError::Io(_) | TransportError::Usb(_)) | DeviceError::Usb(_))
    )
}

/// Waits up to 60 s for the Zune to reappear, then reconnects and re-authenticates.
fn reconnect(keys: &MtpzKeys) -> Result<Zune> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        std::thread::sleep(std::time::Duration::from_secs(2));
        match Zune::connect(Some(keys)) {
            Ok(z) => return Ok(z),
            Err(e) if std::time::Instant::now() < deadline => {
                let _ = e;
            }
            Err(e) => return Err(e).context("the Zune did not come back within 60 s"),
        }
    }
}

fn play(paths: Vec<PathBuf>, album: Option<String>, volume: f32, seconds: Option<f64>, seek_test: Option<u64>) -> Result<()> {
    use squirt_player::{Event, Player, Status};
    let (queue, titles): (Vec<PathBuf>, Vec<String>) = if let Some(name) = album {
        let lib = open_library()?;
        let a = lib
            .albums(None)?
            .into_iter()
            .find(|a| a.title.to_lowercase().contains(&name.to_lowercase()))
            .with_context(|| format!("no album matching {name:?}"))?;
        println!("{} - {}", a.artist, a.title);
        lib.album_tracks(&a.artist, &a.title)?.into_iter().map(|t| (t.path, t.title)).unzip()
    } else {
        let files = collect_audio(&paths)?;
        let titles = files.iter().map(|f| f.file_stem().unwrap_or_default().to_string_lossy().into_owned()).collect();
        (files, titles)
    };
    anyhow::ensure!(!queue.is_empty(), "nothing to play");
    let player = Player::new()?;
    player.set_volume(volume);
    player.play_queue(queue, 0);
    let t0 = std::time::Instant::now();
    let mut seeked = false;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
        for e in player.take_events() {
            match e {
                Event::TrackStarted(i) => println!("\n> {}", titles[i]),
                Event::TrackCompleted(i) => println!("\n  completed {}", titles[i]),
                Event::TrackFailed(i, err) => println!("\n  skipped {}: {err}", titles[i]),
                Event::QueueEnded => {
                    println!("\nend of queue");
                    return Ok(());
                }
            }
        }
        let s = player.state();
        print!(
            "\r  {:?} {}:{:02} / {}:{:02}   ",
            s.status,
            s.position_ms / 60000,
            s.position_ms / 1000 % 60,
            s.duration_ms / 60000,
            s.duration_ms / 1000 % 60
        );
        use std::io::Write;
        std::io::stdout().flush().ok();
        if let (Some(target), false) = (seek_test, seeked) {
            if t0.elapsed().as_secs_f64() > 1.0 {
                player.seek(target * 1000);
                seeked = true;
            }
        }
        if seconds.is_some_and(|max| t0.elapsed().as_secs_f64() >= max) {
            println!();
            return Ok(());
        }
        if s.status == Status::Stopped && t0.elapsed().as_secs() > 2 {
            println!();
            return Ok(());
        }
    }
}
