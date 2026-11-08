//! MTPZ key material.
//!
//! Squirt does not ship keys. Users supply a file in libmtp's `.mtpz-data`
//! format: five hex-encoded lines, in this order: public exponent,
//! encryption key, modulus, private key, certificate.
//!
//! Still to come: the handshake itself (RSA-signed app certificate exchange,
//! then AES session setup), modeled on libmtp's `mtpz.c`.

use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum KeyError {
    #[error("could not read key file {0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error("key file has {0} non-empty lines, expected 5")]
    LineCount(usize),
    #[error("line {line} ({field}) is not valid hex")]
    Hex { line: usize, field: &'static str },
}

pub struct MtpzKeys {
    pub public_exponent: Vec<u8>,
    pub encryption_key: Vec<u8>,
    pub modulus: Vec<u8>,
    pub private_key: Vec<u8>,
    pub certificate: Vec<u8>,
}

impl std::fmt::Debug for MtpzKeys {
    // Never print key material.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MtpzKeys")
            .field("modulus_bits", &(self.modulus.len() * 8))
            .field("certificate_len", &self.certificate.len())
            .finish_non_exhaustive()
    }
}

const FIELDS: [&str; 5] = ["public exponent", "encryption key", "modulus", "private key", "certificate"];

impl MtpzKeys {
    pub fn parse(text: &str) -> Result<Self, KeyError> {
        let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        if lines.len() != 5 {
            return Err(KeyError::LineCount(lines.len()));
        }
        let mut v = lines.iter().enumerate().map(|(i, l)| {
            hex::decode(l).map_err(|_| KeyError::Hex { line: i + 1, field: FIELDS[i] })
        });
        Ok(Self {
            public_exponent: v.next().unwrap()?,
            encryption_key: v.next().unwrap()?,
            modulus: v.next().unwrap()?,
            private_key: v.next().unwrap()?,
            certificate: v.next().unwrap()?,
        })
    }

    pub fn load(path: &Path) -> Result<Self, KeyError> {
        let text = std::fs::read_to_string(path).map_err(|e| KeyError::Io(path.into(), e))?;
        Self::parse(&text)
    }

    /// Default location, shared with libmtp: `~/.mtpz-data`.
    pub fn default_path() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(|h| PathBuf::from(h).join(".mtpz-data"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_five_hex_lines() {
        let k = MtpzKeys::parse("010001\nAABB\nCCDD\nEEFF\n0011\n").unwrap();
        assert_eq!(k.public_exponent, vec![1, 0, 1]);
        assert_eq!(k.certificate, vec![0, 0x11]);
    }

    #[test]
    fn rejects_wrong_line_count() {
        assert!(matches!(MtpzKeys::parse("00\n11\n"), Err(KeyError::LineCount(2))));
    }

    #[test]
    fn names_bad_hex_field() {
        let err = MtpzKeys::parse("00\n00\nZZ\n00\n00").unwrap_err();
        assert!(matches!(err, KeyError::Hex { line: 3, field: "modulus" }));
    }
}
