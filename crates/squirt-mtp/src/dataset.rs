//! PTP dataset decoding (PIMA 15740 section 5.3), starting with DeviceInfo.

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
#[error("dataset truncated at byte {0}")]
pub struct Truncated(pub usize);

/// Little-endian cursor over PTP dataset bytes.
pub struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], Truncated> {
        let s = self.buf.get(self.pos..self.pos + n).ok_or(Truncated(self.pos))?;
        self.pos += n;
        Ok(s)
    }
    pub fn u8(&mut self) -> Result<u8, Truncated> {
        Ok(self.take(1)?[0])
    }
    pub fn u16(&mut self) -> Result<u16, Truncated> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    pub fn u32(&mut self) -> Result<u32, Truncated> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    pub fn u64(&mut self) -> Result<u64, Truncated> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    /// A u32 count followed by that many u16 values.
    pub fn u16_array(&mut self) -> Result<Vec<u16>, Truncated> {
        let n = self.u32()? as usize;
        (0..n).map(|_| self.u16()).collect()
    }
    /// A u8 count of UTF-16 code units (including the trailing NUL), then UTF-16LE.
    pub fn string(&mut self) -> Result<String, Truncated> {
        let n = self.u8()? as usize;
        let units: Vec<u16> = (0..n).map(|_| self.u16()).collect::<Result<_, _>>()?;
        let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
        Ok(String::from_utf16_lossy(&units[..end]))
    }
}

/// Encodes a PTP string: u8 count of UTF-16 units including NUL, then UTF-16LE.
/// The empty string is a single zero byte.
pub fn encode_string(s: &str) -> Vec<u8> {
    if s.is_empty() {
        return vec![0];
    }
    let units: Vec<u16> = s.encode_utf16().chain([0]).take(255).collect();
    let mut out = vec![units.len() as u8];
    units.iter().for_each(|u| out.extend(u.to_le_bytes()));
    out
}

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub standard_version: u16,
    pub vendor_extension_id: u32,
    pub vendor_extension_version: u16,
    pub vendor_extension_desc: String,
    pub functional_mode: u16,
    pub operations: Vec<u16>,
    pub events: Vec<u16>,
    pub device_properties: Vec<u16>,
    pub capture_formats: Vec<u16>,
    pub playback_formats: Vec<u16>,
    pub manufacturer: String,
    pub model: String,
    pub device_version: String,
    pub serial_number: String,
}

impl DeviceInfo {
    pub fn parse(buf: &[u8]) -> Result<Self, Truncated> {
        let mut r = Reader::new(buf);
        Ok(Self {
            standard_version: r.u16()?,
            vendor_extension_id: r.u32()?,
            vendor_extension_version: r.u16()?,
            vendor_extension_desc: r.string()?,
            functional_mode: r.u16()?,
            operations: r.u16_array()?,
            events: r.u16_array()?,
            device_properties: r.u16_array()?,
            capture_formats: r.u16_array()?,
            playback_formats: r.u16_array()?,
            manufacturer: r.string()?,
            model: r.string()?,
            device_version: r.string()?,
            serial_number: r.string()?,
        })
    }

    pub fn supports(&self, op: u16) -> bool {
        self.operations.contains(&op)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ptp_str(s: &str, out: &mut Vec<u8>) {
        out.extend(encode_string(s));
    }

    #[test]
    fn string_roundtrip() {
        let b = encode_string("Squirt");
        assert_eq!(b[0], 7);
        assert_eq!(Reader::new(&b).string().unwrap(), "Squirt");
    }

    #[test]
    fn parses_device_info() {
        let mut b = Vec::new();
        b.extend(100u16.to_le_bytes());
        b.extend(6u32.to_le_bytes());
        b.extend(100u16.to_le_bytes());
        ptp_str("microsoft.com: 1.0", &mut b);
        b.extend(0u16.to_le_bytes());
        b.extend(2u32.to_le_bytes());
        b.extend(0x1001u16.to_le_bytes());
        b.extend(0x9212u16.to_le_bytes());
        for _ in 0..4 {
            b.extend(0u32.to_le_bytes());
        }
        for s in ["Microsoft", "Zune HD", "4.5", ""] {
            ptp_str(s, &mut b);
        }
        let d = DeviceInfo::parse(&b).unwrap();
        assert_eq!(d.vendor_extension_desc, "microsoft.com: 1.0");
        assert_eq!(d.model, "Zune HD");
        assert_eq!(d.serial_number, "");
        assert!(d.supports(0x9212));
    }

    #[test]
    fn truncation_is_an_error() {
        assert_eq!(DeviceInfo::parse(&[1, 0, 0]).unwrap_err(), Truncated(2));
    }
}
