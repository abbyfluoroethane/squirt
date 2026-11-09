use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use squirt_mtp::{container::op, dataset::DeviceInfo, devices, mtpz::{self, MtpzKeys}, transport::Transport};
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
