//! A connected, authenticated Zune: the API the CLI and app build on.
//! Operations and datasets follow the public MTP 1.1 specification.

use crate::container::op;
use crate::dataset::{DeviceInfo, Reader, Truncated};
use crate::devices::{self, FoundZune};
use crate::mtpz::{self, MtpzError, MtpzKeys};
use crate::transport::{Transport, TransportError};
use std::collections::HashMap;
use thiserror::Error;

/// Parent handle meaning "root of the storage".
pub const ROOT: u32 = 0xFFFF_FFFF;

pub mod format {
    pub const UNDEFINED: u16 = 0x3000;
    pub const ASSOCIATION: u16 = 0x3001; // folder
    pub const MP3: u16 = 0x3009;
    pub const WMA: u16 = 0xB901;
    /// AAC in an MPEG-4 container (.m4a). Same code as WPD's M4A format.
    pub const M4A: u16 = 0xB215;
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
    /// The Zune HD only accepts 0 or 1 here, so multi-disc albums cannot use it.
    pub const DISC_NUMBER: u16 = 0xDC9D;
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

/// A decoded property value of any MTP datatype.
#[derive(Debug, Clone, PartialEq)]
pub enum AnyValue {
    Int(i128),
    Str(String),
    Array(Vec<i128>),
    U128([u8; 16]),
}

impl std::fmt::Display for AnyValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(v) => write!(f, "{v} ({v:#x})"),
            Self::Str(s) => write!(f, "{s:?}"),
            Self::Array(a) if a.len() > 32 => write!(f, "[{} items]", a.len()),
            Self::Array(a) => write!(f, "{a:?}"),
            Self::U128(b) => write!(f, "{}", b.iter().rev().map(|x| format!("{x:02x}")).collect::<String>()),
        }
    }
}

fn read_scalar(r: &mut Reader, dt: u16) -> Result<i128, Truncated> {
    Ok(match dt & 0x0fff {
        0x1 => r.u8()? as i8 as i128,
        0x2 => r.u8()? as i128,
        0x3 => r.u16()? as i16 as i128,
        0x4 => r.u16()? as i128,
        0x5 => r.u32()? as i32 as i128,
        0x6 => r.u32()? as i128,
        0x7 => r.u64()? as i64 as i128,
        0x8 => r.u64()? as i128,
        _ => return Err(Truncated(0)),
    })
}

/// Reads one value of MTP datatype `dt`.
pub fn read_value(r: &mut Reader, dt: u16) -> Result<AnyValue, Truncated> {
    Ok(match dt {
        0xFFFF => AnyValue::Str(r.string()?),
        0x0009 | 0x000A => {
            let mut b = [0u8; 16];
            for x in b.iter_mut() {
                *x = r.u8()?;
            }
            AnyValue::U128(b)
        }
        d if d & 0x4000 != 0 => {
            let n = r.u32()?;
            AnyValue::Array((0..n).map(|_| read_scalar(r, d)).collect::<Result<_, _>>()?)
        }
        d => AnyValue::Int(read_scalar(r, d)?),
    })
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
    /// parent handle -> (file name -> handle), filled lazily per folder.
    names: HashMap<u32, HashMap<String, u32>>,
    /// (folder, format) -> ((title, track number) -> handle), filled lazily.
    tracks: HashMap<(u32, u16), HashMap<(String, u16), u32>>,
}

fn file_name_of(props: &[(u16, PropValue)]) -> Option<&str> {
    props.iter().find_map(|(c, v)| match (c, v) {
        (&prop::OBJECT_FILE_NAME, PropValue::Str(s)) => Some(s.as_str()),
        _ => None,
    })
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
        let mut z = Self { found, info, t, names: HashMap::new(), tracks: HashMap::new() };
        // List storage before the handshake, as the reference flow does.
        z.storage_ids()?;
        if let Some(keys) = keys {
            if mtpz::advertises_mtpz(&z.info.vendor_extension_desc) {
                mtpz::authenticate(&mut z.t, keys)?;
            }
        }
        Ok(z)
    }

    /// The underlying transport, for modules that add device operations.
    pub(crate) fn transport(&mut self) -> &mut Transport {
        &mut self.t
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

    /// Runs a read-only operation and returns its raw data phase (for debugging).
    pub fn raw(&mut self, code: u16, params: &[u32]) -> Result<Vec<u8>, DeviceError> {
        Ok(self.t.transact(code, params, None)?.data.unwrap_or_default())
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

    pub fn prop_u32(&mut self, handle: u32, code: u16) -> Result<u32, DeviceError> {
        let d = self.data(op::GET_OBJECT_PROP_VALUE, &[handle, code as u32], "ObjectPropValue")?;
        let mut r = Reader::new(&d);
        // Accept u16 values too (for example ObjectFormat).
        Ok(if d.len() == 2 { r.u16()? as u32 } else { r.u32()? })
    }

    pub fn prop_u64(&mut self, handle: u32, code: u16) -> Result<u64, DeviceError> {
        let d = self.data(op::GET_OBJECT_PROP_VALUE, &[handle, code as u32], "ObjectPropValue")?;
        Ok(Reader::new(&d).u64()?)
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
        self.send_object_progress(storage, parent, format, props, bytes, &mut |_| {})
    }

    /// Like [`Self::send_object`], reporting bytes sent through `progress`.
    pub fn send_object_progress(
        &mut self,
        storage: u32,
        parent: u32,
        format: u16,
        props: &[(u16, PropValue)],
        bytes: &[u8],
        progress: &mut dyn FnMut(usize),
    ) -> Result<u32, DeviceError> {
        let size = bytes.len() as u64;
        let reply = self.t.transact(
            op::SEND_OBJECT_PROP_LIST,
            &[storage, parent, format as u32, (size >> 32) as u32, size as u32],
            Some(&encode_prop_list(0, props)),
        )?;
        let handle = *reply.params.get(2).ok_or(DeviceError::Unexpected("no handle from SendObjectPropList"))?;
        self.t.transact_progress(op::SEND_OBJECT, &[], Some(bytes), progress)?;
        if let Some(name) = file_name_of(props) {
            self.remember(parent, name, handle);
        }
        Ok(handle)
    }

    /// Records an object created by another module.
    pub(crate) fn note_created(&mut self, parent: u32, props: &[(u16, PropValue)], handle: u32) {
        if let Some(name) = file_name_of(props) {
            self.remember(parent, name, handle);
        }
    }

    /// All properties of one object, via GetObjectPropList (all props, depth 0).
    pub fn all_props(&mut self, handle: u32) -> Result<Vec<(u16, u16, AnyValue)>, DeviceError> {
        let d = self.data(op::GET_OBJECT_PROP_LIST, &[handle, 0, 0xFFFF_FFFF, 0, 0], "ObjectPropList")?;
        let mut r = Reader::new(&d);
        let n = r.u32()?;
        let mut out = Vec::new();
        for _ in 0..n {
            r.u32()?; // handle
            let code = r.u16()?;
            let dt = r.u16()?;
            out.push((code, dt, read_value(&mut r, dt)?));
        }
        Ok(out)
    }

    /// Raw bytes of an array-of-u8 property (for example album art, 0xDC86).
    pub fn prop_bytes(&mut self, handle: u32, code: u16) -> Result<Vec<u8>, DeviceError> {
        let d = self.data(op::GET_OBJECT_PROP_VALUE, &[handle, code as u32], "ObjectPropValue")?;
        let mut r = Reader::new(&d);
        let n = r.u32()? as usize;
        Ok((0..n).map(|_| r.u8()).collect::<Result<_, _>>()?)
    }

    pub fn references(&mut self, handle: u32) -> Result<Vec<u32>, DeviceError> {
        self.u32_array(op::GET_OBJECT_REFERENCES, &[handle], "ObjectReferences")
    }

    /// Loads the file names of `parent`'s children into the cache. Uses one
    /// GetObjectPropList (depth 1) for folders, and per-object lookups at the root.
    fn load_names(&mut self, storage: u32, parent: u32) -> Result<(), DeviceError> {
        if self.names.contains_key(&parent) {
            return Ok(());
        }
        let mut map = HashMap::new();
        if parent == ROOT {
            for h in self.object_handles(storage, 0, ROOT)? {
                map.insert(self.prop_string(h, prop::OBJECT_FILE_NAME)?, h);
            }
        } else {
            let d = self.data(
                op::GET_OBJECT_PROP_LIST,
                &[parent, 0, prop::OBJECT_FILE_NAME as u32, 0, 1],
                "ObjectPropList",
            )?;
            let mut r = Reader::new(&d);
            for _ in 0..r.u32()? {
                let h = r.u32()?;
                let code = r.u16()?;
                let dt = r.u16()?;
                let v = read_value(&mut r, dt)?;
                if let (true, true, AnyValue::Str(name)) = (h != parent, code == prop::OBJECT_FILE_NAME, v) {
                    map.insert(name, h);
                }
            }
        }
        self.names.insert(parent, map);
        Ok(())
    }

    fn remember(&mut self, parent: u32, name: &str, handle: u32) {
        if let Some(m) = self.names.get_mut(&parent) {
            m.insert(name.to_string(), handle);
        }
    }

    /// Finds a track of `format` in folder `parent` by title and track number.
    /// The device renames some files itself, so file names cannot be used.
    ///
    /// The Zune HD refuses GetObjectPropList for a single property code at
    /// depth 1 (0xA801), except for the file name. It does answer by property
    /// group: title (0xDC44) and track number (0xDC8B) are both in group 3.
    pub fn find_track(&mut self, parent: u32, format: u16, title: &str, track: u16) -> Result<Option<u32>, DeviceError> {
        let key = (parent, format);
        if !self.tracks.contains_key(&key) {
            let d = self.data(op::GET_OBJECT_PROP_LIST, &[parent, format as u32, 0, 3, 1], "ObjectPropList")?;
            let mut r = Reader::new(&d);
            let mut names: HashMap<u32, String> = HashMap::new();
            let mut numbers: HashMap<u32, u16> = HashMap::new();
            for _ in 0..r.u32()? {
                let h = r.u32()?;
                let code = r.u16()?;
                let dt = r.u16()?;
                match (code, read_value(&mut r, dt)?) {
                    (prop::NAME, AnyValue::Str(n)) if h != parent => {
                        names.insert(h, n);
                    }
                    (prop::TRACK, AnyValue::Int(t)) if h != parent => {
                        numbers.insert(h, t as u16);
                    }
                    _ => {}
                }
            }
            let map = names
                .into_iter()
                .map(|(h, n)| ((n, numbers.get(&h).copied().unwrap_or(0)), h))
                .collect();
            self.tracks.insert(key, map);
        }
        Ok(self.tracks[&key].get(&(title.to_string(), track)).copied())
    }

    pub(crate) fn note_track(&mut self, parent: u32, format: u16, title: &str, track: u16, handle: u32) {
        if let Some(m) = self.tracks.get_mut(&(parent, format)) {
            m.insert((title.to_string(), track), handle);
        }
    }

    /// Finds a child of `parent` by file name.
    pub fn find_child(&mut self, storage: u32, parent: u32, name: &str) -> Result<Option<u32>, DeviceError> {
        self.load_names(storage, parent)?;
        Ok(self.names[&parent].get(name).copied())
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
        let h = reply.params.get(2).copied().ok_or(DeviceError::Unexpected("no handle for new folder"))?;
        self.remember(parent, name, h);
        self.names.insert(h, HashMap::new()); // A new folder is empty.

        Ok(h)
    }

    pub fn delete_object(&mut self, handle: u32) -> Result<(), DeviceError> {
        self.t.transact(op::DELETE_OBJECT, &[handle, 0], None)?;
        // The device may also delete linked objects, so drop the whole cache.
        self.names.clear();
        self.tracks.clear();
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
