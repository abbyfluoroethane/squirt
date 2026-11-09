//! PTP/MTP USB container format (PIMA 15740 / MTP 1.1 USB transport).
//!
//! Header, little-endian: length u32 | type u16 | code u16 | transaction_id u32,
//! then up to 5 u32 parameters (command/response) or a raw payload (data).

use thiserror::Error;

pub const HEADER_LEN: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ContainerType {
    Command = 1,
    Data = 2,
    Response = 3,
    Event = 4,
}

impl TryFrom<u16> for ContainerType {
    type Error = ContainerError;
    fn try_from(v: u16) -> Result<Self, Self::Error> {
        Ok(match v {
            1 => Self::Command,
            2 => Self::Data,
            3 => Self::Response,
            4 => Self::Event,
            other => return Err(ContainerError::BadType(other)),
        })
    }
}

/// Operation codes used so far.
pub mod op {
    pub const GET_DEVICE_INFO: u16 = 0x1001;
    pub const OPEN_SESSION: u16 = 0x1002;
    pub const CLOSE_SESSION: u16 = 0x1003;
    pub const GET_STORAGE_IDS: u16 = 0x1004;
    pub const GET_STORAGE_INFO: u16 = 0x1005;
    pub const GET_OBJECT_HANDLES: u16 = 0x1007;
    pub const GET_OBJECT_INFO: u16 = 0x1008;
    pub const GET_OBJECT: u16 = 0x1009;
    pub const DELETE_OBJECT: u16 = 0x100B;
    pub const SEND_OBJECT: u16 = 0x100D;
    pub const SET_DEVICE_PROP_VALUE: u16 = 0x1016;
    pub const GET_OBJECT_PROPS_SUPPORTED: u16 = 0x9801;
    pub const GET_OBJECT_PROP_VALUE: u16 = 0x9803;
    pub const SET_OBJECT_PROP_VALUE: u16 = 0x9804;
    pub const SEND_OBJECT_PROP_LIST: u16 = 0x9808;
    pub const SET_OBJECT_REFERENCES: u16 = 0x9811;
    // MTPZ / WMDRMPD vendor ops (docs/mtpz-protocol.md section 2).
    pub const WMDRMPD_SEND_APP_REQUEST: u16 = 0x9212;
    pub const WMDRMPD_GET_APP_RESPONSE: u16 = 0x9213;
    pub const ENABLE_TRUSTED_FILES_OPERATIONS: u16 = 0x9214;
    pub const END_TRUSTED_APP_SESSION: u16 = 0x9216;
}

pub const RESPONSE_OK: u16 = 0x2001;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ContainerError {
    #[error("container too short ({0} bytes)")]
    Short(usize),
    #[error("length field {declared} disagrees with buffer {actual}")]
    LengthMismatch { declared: u32, actual: usize },
    #[error("unknown container type {0:#06x}")]
    BadType(u16),
    #[error("too many parameters ({0}, max 5)")]
    TooManyParams(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container {
    pub kind: ContainerType,
    pub code: u16,
    pub transaction_id: u32,
    pub payload: Vec<u8>,
}

impl Container {
    pub fn command(code: u16, transaction_id: u32, params: &[u32]) -> Result<Self, ContainerError> {
        if params.len() > 5 {
            return Err(ContainerError::TooManyParams(params.len()));
        }
        Ok(Self {
            kind: ContainerType::Command,
            code,
            transaction_id,
            payload: params.iter().flat_map(|p| p.to_le_bytes()).collect(),
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let len = (HEADER_LEN + self.payload.len()) as u32;
        let mut out = Vec::with_capacity(len as usize);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(self.kind as u16).to_le_bytes());
        out.extend_from_slice(&self.code.to_le_bytes());
        out.extend_from_slice(&self.transaction_id.to_le_bytes());
        out.extend_from_slice(&self.payload);
        out
    }

    pub fn decode(buf: &[u8]) -> Result<Self, ContainerError> {
        if buf.len() < HEADER_LEN {
            return Err(ContainerError::Short(buf.len()));
        }
        let u16_at = |i: usize| u16::from_le_bytes([buf[i], buf[i + 1]]);
        let u32_at = |i: usize| u32::from_le_bytes(buf[i..i + 4].try_into().unwrap());
        let declared = u32_at(0);
        if declared as usize != buf.len() {
            return Err(ContainerError::LengthMismatch { declared, actual: buf.len() });
        }
        Ok(Self {
            kind: ContainerType::try_from(u16_at(4))?,
            code: u16_at(6),
            transaction_id: u32_at(8),
            payload: buf[HEADER_LEN..].to_vec(),
        })
    }

    /// The payload read as u32 parameters (command/response containers).
    pub fn params(&self) -> Vec<u32> {
        self.payload
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_session_roundtrip() {
        let c = Container::command(op::OPEN_SESSION, 0, &[1]).unwrap();
        let bytes = c.encode();
        assert_eq!(bytes, [16, 0, 0, 0, 1, 0, 0x02, 0x10, 0, 0, 0, 0, 1, 0, 0, 0]);
        let back = Container::decode(&bytes).unwrap();
        assert_eq!(back, c);
        assert_eq!(back.params(), vec![1]);
    }

    #[test]
    fn rejects_bad_length() {
        let mut bytes = Container::command(op::GET_DEVICE_INFO, 1, &[]).unwrap().encode();
        bytes[0] = 99;
        assert!(matches!(Container::decode(&bytes), Err(ContainerError::LengthMismatch { .. })));
    }
}
