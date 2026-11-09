//! A connected, authenticated Zune: the API the CLI and app build on.
//! Operations and datasets follow the public MTP 1.1 specification.

use crate::container::op;
use crate::dataset::{DeviceInfo, Reader, Truncated};
use crate::devices::{self, FoundZune};
use crate::mtpz::{self, MtpzError, MtpzKeys};
use crate::transport::{Transport, TransportError};
use thiserror::Error;

/// Parent handle meaning "root of the storage".
pub const ROOT: u32 = 0xFFFF_FFFF;

pub mod format {
    pub const UNDEFINED: u16 = 0x3000;
    pub const ASSOCIATION: u16 = 0x3001; // folder
    pub const MP3: u16 = 0x3009;
    pub const WMA: u16 = 0xB901;
    pub const ABSTRACT_AUDIO_ALBUM: u16 = 0xBA03;
}

pub mod prop {
    pub const STORAGE_ID: u16 = 0xDC01;
    pub const OBJECT_FORMAT: u16 = 0xDC02;
    pub const OBJECT_SIZE: u16 = 0xDC04;
    pub const OBJECT_FILE_NAME: u16 = 0xDC07;
    pub const PARENT_OBJECT: u16 = 0xDC0B;
    pub const NAME: u16 = 0xDC44;
    pub const ARTIST: u16 = 0xDC46;
    pub const DURATION: u16 = 0xDC89;
    pub const TRACK: u16 = 0xDC8B;
    pub const GENRE: u16 = 0xDC8C;
    pub const ORIGINAL_RELEASE_DATE: u16 = 0xDC99;
    pub const ALBUM_NAME: u16 = 0xDC9A;
    pub const ALBUM_ARTIST: u16 = 0xDC9B;
}

#[derive(Debug, Error)]
pub enum DeviceError {
    #[error("no Zune found")]
    NotFound,
    #[error(transparent)]
    Usb(#[from] nusb::Error),
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    Mtpz(#[from] MtpzError),
    #[error(transparent)]
    Dataset(#[from] Truncated),
    #[error("device sent no data for {0}")]
    NoData(&'static str),
    #[error("unexpected response: {0}")]
    Unexpected(&'static str),
}

#[derive(Debug, Clone)]
pub struct StorageInfo {
    pub id: u32,
    pub storage_type: u16,
    pub filesystem_type: u16,
    pub access: u16,
    pub max_capacity: u64,
    pub free_bytes: u64,
    pub description: String,
    pub volume_id: String,
}

/// A typed MTP property value, for SendObjectPropList.
#[derive(Debug, Clone)]
pub enum PropValue {
    U16(u16),
    U32(u32),
    U64(u64),
    Str(String),
}

impl PropValue {
    fn datatype(&self) -> u16 {
        match self {
            Self::U16(_) => 0x0004,
            Self::U32(_) => 0x0006,
            Self::U64(_) => 0x0008,
            Self::Str(_) => 0xFFFF,
        }
    }
    fn encode(&self, out: &mut Vec<u8>) {
        match self {
            Self::U16(v) => out.extend(v.to_le_bytes()),
            Self::U32(v) => out.extend(v.to_le_bytes()),
            Self::U64(v) => out.extend(v.to_le_bytes()),
            Self::Str(s) => out.extend(crate::dataset::encode_string(s)),
        }
    }
}

/// Encodes an ObjectPropList dataset (MTP 1.1 section 5.4.3).
pub fn encode_prop_list(handle: u32, props: &[(u16, PropValue)]) -> Vec<u8> {
    let mut out = (props.len() as u32).to_le_bytes().to_vec();
    for (code, value) in props {
        out.extend(handle.to_le_bytes());
        out.extend(code.to_le_bytes());
        out.extend(value.datatype().to_le_bytes());
        value.encode(&mut out);
    }
    out
}

pub struct Zune {
    pub found: FoundZune,
    pub info: DeviceInfo,
    t: Transport,
}

impl Zune {
    /// Finds the first Zune, opens a session and, when the device needs it and
    /// keys are given, runs the MTPZ handshake.
    pub fn connect(keys: Option<&MtpzKeys>) -> Result<Self, DeviceError> {
        let found = devices::find_zunes()?.into_iter().next().ok_or(DeviceError::NotFound)?;
        let mut t = Transport::open(&found.info)?;
        let data = t.transact(op::GET_DEVICE_INFO, &[], None)?.data;
        let info = DeviceInfo::parse(&data.ok_or(DeviceError::NoData("DeviceInfo"))?)?;
        t.open_session(1)?;
        let mut z = Self { found, info, t };
        // List storage before the handshake, as the reference flow does.
        z.storage_ids()?;
        if let Some(keys) = keys {
            if mtpz::advertises_mtpz(&z.info.vendor_extension_desc) {
                mtpz::authenticate(&mut z.t, keys)?;
            }
        }
        Ok(z)
    }

    fn data(&mut self, code: u16, params: &[u32], what: &'static str) -> Result<Vec<u8>, DeviceError> {
        self.t.transact(code, params, None)?.data.ok_or(DeviceError::NoData(what))
    }

    fn u32_array(&mut self, code: u16, params: &[u32], what: &'static str) -> Result<Vec<u32>, DeviceError> {
        let d = self.data(code, params, what)?;
        let mut r = Reader::new(&d);
        let n = r.u32()?;
        Ok((0..n).map(|_| r.u32()).collect::<Result<_, _>>()?)
    }

    pub fn storage_ids(&mut self) -> Result<Vec<u32>, DeviceError> {
        self.u32_array(op::GET_STORAGE_IDS, &[], "StorageIDs")
    }

    pub fn storage_info(&mut self, id: u32) -> Result<StorageInfo, DeviceError> {
        let d = self.data(op::GET_STORAGE_INFO, &[id], "StorageInfo")?;
        let mut r = Reader::new(&d);
        Ok(StorageInfo {
            id,
            storage_type: r.u16()?,
            filesystem_type: r.u16()?,
            access: r.u16()?,
            max_capacity: r.u64()?,
            free_bytes: r.u64()?,
            description: {
                r.u32()?; // FreeSpaceInObjects
                r.string()?
            },
            volume_id: r.string()?,
        })
    }

    /// Object handles in `storage` under `parent` (use [`ROOT`]), optionally one format.
    pub fn object_handles(&mut self, storage: u32, format: u16, parent: u32) -> Result<Vec<u32>, DeviceError> {
        self.u32_array(op::GET_OBJECT_HANDLES, &[storage, format as u32, parent], "ObjectHandles")
    }

    pub fn prop_string(&mut self, handle: u32, code: u16) -> Result<String, DeviceError> {
        let d = self.data(op::GET_OBJECT_PROP_VALUE, &[handle, code as u32], "ObjectPropValue")?;
        Ok(Reader::new(&d).string()?)
    }

    pub fn prop_u16(&mut self, handle: u32, code: u16) -> Result<u16, DeviceError> {
        let d = self.data(op::GET_OBJECT_PROP_VALUE, &[handle, code as u32], "ObjectPropValue")?;
        Ok(Reader::new(&d).u16()?)
    }

    pub fn props_supported(&mut self, format: u16) -> Result<Vec<u16>, DeviceError> {
        let d = self.data(op::GET_OBJECT_PROPS_SUPPORTED, &[format as u32], "ObjectPropsSupported")?;
        Ok(Reader::new(&d).u16_array()?)
    }

    /// Creates an object with SendObjectPropList, then sends its bytes with
    /// SendObject. Returns the new object handle.
    pub fn send_object(
        &mut self,
        storage: u32,
        parent: u32,
        format: u16,
        props: &[(u16, PropValue)],
        bytes: &[u8],
    ) -> Result<u32, DeviceError> {
        let size = bytes.len() as u64;
        let reply = self.t.transact(
            op::SEND_OBJECT_PROP_LIST,
            &[storage, parent, format as u32, (size >> 32) as u32, size as u32],
            Some(&encode_prop_list(0, props)),
        )?;
        let handle = *reply.params.get(2).ok_or(DeviceError::Unexpected("no handle from SendObjectPropList"))?;
        self.t.transact(op::SEND_OBJECT, &[], Some(bytes))?;
        Ok(handle)
    }

    /// Finds a child of `parent` by file name.
    pub fn find_child(&mut self, storage: u32, parent: u32, name: &str) -> Result<Option<u32>, DeviceError> {
        for h in self.object_handles(storage, 0, parent)? {
            if self.prop_string(h, prop::OBJECT_FILE_NAME)? == name {
                return Ok(Some(h));
            }
        }
        Ok(None)
    }

    /// Returns the folder `name` under `parent`, creating it if needed.
    pub fn ensure_folder(&mut self, storage: u32, parent: u32, name: &str) -> Result<u32, DeviceError> {
        if let Some(h) = self.find_child(storage, parent, name)? {
            return Ok(h);
        }
        let reply = self.t.transact(
            op::SEND_OBJECT_PROP_LIST,
            &[storage, parent, format::ASSOCIATION as u32, 0, 0],
            Some(&encode_prop_list(0, &[(prop::OBJECT_FILE_NAME, PropValue::Str(name.into()))])),
        )?;
        reply.params.get(2).copied().ok_or(DeviceError::Unexpected("no handle for new folder"))
    }

    pub fn delete_object(&mut self, handle: u32) -> Result<(), DeviceError> {
        self.t.transact(op::DELETE_OBJECT, &[handle, 0], None)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prop_list_layout() {
        let b = encode_prop_list(0, &[(prop::TRACK, PropValue::U16(3)), (prop::NAME, PropValue::Str("A".into()))]);
        assert_eq!(
            b,
            [2, 0, 0, 0, /* h */ 0, 0, 0, 0, 0x8b, 0xdc, 4, 0, 3, 0, /* h */ 0, 0, 0, 0, 0x44, 0xdc, 0xff, 0xff, 2, b'A', 0, 0, 0]
        );
    }
}
