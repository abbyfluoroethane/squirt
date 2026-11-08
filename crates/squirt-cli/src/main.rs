use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use squirt_mtp::{devices, mtpz::MtpzKeys};
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
        Cmd::Keys { path } => {
            let path = path
                .or_else(MtpzKeys::default_path)
                .context("no key path given and no home directory found")?;
            let keys = MtpzKeys::load(&path)?;
            println!("OK: {} -> {keys:?}", path.display());
        }
    }
    Ok(())
}
