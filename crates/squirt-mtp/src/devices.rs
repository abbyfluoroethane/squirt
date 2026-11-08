//! Zune USB identification. IDs are from libmtp's `music-players.h`.

pub const MICROSOFT_VID: u16 = 0x045e;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZuneModel {
    /// Zune HD (16/32/64 GB).
    Hd,
    /// Original/flash/HDD Zunes (30, 4/8/16, 80/120). libmtp reports one PID for all of them.
    Classic,
}

impl ZuneModel {
    pub fn from_ids(vid: u16, pid: u16) -> Option<Self> {
        match (vid, pid) {
            (MICROSOFT_VID, 0x063e) => Some(Self::Hd),
            (MICROSOFT_VID, 0x0710) => Some(Self::Classic),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Hd => "Zune HD",
            Self::Classic => "Zune (classic/flash)",
        }
    }
}

#[derive(Debug, Clone)]
pub struct FoundZune {
    pub model: ZuneModel,
    pub serial: Option<String>,
    pub info: nusb::DeviceInfo,
}

/// Lists every connected Zune.
pub fn find_zunes() -> Result<Vec<FoundZune>, nusb::Error> {
    use nusb::MaybeFuture;
    Ok(nusb::list_devices()
        .wait()?
        .filter_map(|info| {
            let model = ZuneModel::from_ids(info.vendor_id(), info.product_id())?;
            Some(FoundZune {
                model,
                serial: info.serial_number().map(str::to_owned),
                info,
            })
        })
        .collect())
}
