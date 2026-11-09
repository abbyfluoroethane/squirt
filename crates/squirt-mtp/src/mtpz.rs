//! MTPZ: key material and the trust handshake.
//!
//! Clean-room implementation written only from `docs/mtpz-protocol.md`.
//! Do not consult libmtp or any other GPL/LGPL source when changing this file.
//!
//! Keys come in the `.mtpz-data` format: five hex lines, in this order:
//! public exponent, encryption key (unused), modulus, private exponent,
//! certificate chain. A file at `~/.mtpz-data` is used if present; otherwise
//! the keys built in from the `SQUIRT_MTPZ_DATA` environment variable at
//! compile time (set in the git-ignored `.cargo/config.toml`, or from a CI
//! secret). The keys themselves are never in the repository.

use crate::container::op;
use crate::dataset;
use crate::transport::{Transport, TransportError};
use aes::cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit};
use aes::Aes128;
use cmac::{Cmac, Mac};
use num_bigint::BigUint;
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use thiserror::Error;

const RSA_LEN: usize = 128;
const SESSION_INITIATOR_INFO: u32 = 0xD406;
const INITIATOR_STRING: &str = "Squirt/MTPZClassDriver";

// ---------------------------------------------------------------- key file

#[derive(Debug, Error)]
pub enum KeyError {
    #[error("could not read key file {0}: {1}")]
    Io(PathBuf, std::io::Error),
    #[error("key file has {0} non-empty lines, expected 5")]
    LineCount(usize),
    #[error("line {line} ({field}) is not valid hex")]
    Hex { line: usize, field: &'static str },
    #[error("modulus is {0} bytes, expected 128 (RSA-1024)")]
    ModulusSize(usize),
    #[error("no MTPZ keys: put a key file at ~/.mtpz-data")]
    Missing,
}

/// Key file contents compiled in at build time, if any.
const BUILTIN: Option<&str> = option_env!("SQUIRT_MTPZ_DATA");

pub struct MtpzKeys {
    pub public_exponent: Vec<u8>,
    /// Line 2. Present in the file format, but not used by the handshake (spec section 3, note A).
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
            // Odd-length lines (for example a leading-zero-trimmed exponent) get a leading 0.
            let l = if l.len() % 2 == 1 { format!("0{l}") } else { l.to_string() };
            hex::decode(l).map_err(|_| KeyError::Hex { line: i + 1, field: FIELDS[i] })
        });
        let keys = Self {
            public_exponent: v.next().unwrap()?,
            encryption_key: v.next().unwrap()?,
            modulus: v.next().unwrap()?,
            private_key: v.next().unwrap()?,
            certificate: v.next().unwrap()?,
        };
        let n = BigUint::from_bytes_be(&keys.modulus).to_bytes_be().len();
        if n != RSA_LEN {
            return Err(KeyError::ModulusSize(n));
        }
        Ok(keys)
    }

    pub fn load(path: &Path) -> Result<Self, KeyError> {
        let text = std::fs::read_to_string(path).map_err(|e| KeyError::Io(path.into(), e))?;
        Self::parse(&text)
    }

    /// The keys compiled into this build, if any.
    pub fn builtin() -> Option<Result<Self, KeyError>> {
        BUILTIN.filter(|t| !t.trim().is_empty()).map(Self::parse)
    }

    /// `~/.mtpz-data` if it exists, otherwise the built-in keys.
    pub fn find() -> Result<Self, KeyError> {
        match Self::default_path() {
            Some(p) if p.exists() => Self::load(&p),
            _ => Self::builtin().unwrap_or(Err(KeyError::Missing)),
        }
    }

    /// Default location, shared with other tools: `~/.mtpz-data`.
    pub fn default_path() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(|h| PathBuf::from(h).join(".mtpz-data"))
    }

    /// Raw RSA private operation `x^d mod n`, output left-padded to 128 bytes.
    fn rsa_private(&self, x: &[u8]) -> [u8; RSA_LEN] {
        let n = BigUint::from_bytes_be(&self.modulus);
        let d = BigUint::from_bytes_be(&self.private_key);
        i2osp(&BigUint::from_bytes_be(x).modpow(&d, &n))
    }
}

fn i2osp(x: &BigUint) -> [u8; RSA_LEN] {
    let b = x.to_bytes_be();
    let mut out = [0u8; RSA_LEN];
    out[RSA_LEN - b.len()..].copy_from_slice(&b);
    out
}

// ---------------------------------------------------------------- primitives

fn sha1(parts: &[&[u8]]) -> [u8; 20] {
    let mut h = Sha1::new();
    parts.iter().for_each(|p| h.update(p));
    h.finalize().into()
}

/// MGF1 with SHA-1 (spec, notation).
fn mgf1(seed: &[u8], len: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(len + 20);
    let mut counter = 0u32;
    while out.len() < len {
        out.extend_from_slice(&sha1(&[seed, &counter.to_be_bytes()]));
        counter += 1;
    }
    out.truncate(len);
    out
}

fn xor_in_place(a: &mut [u8], b: &[u8]) {
    a.iter_mut().zip(b).for_each(|(x, y)| *x ^= y);
}

fn aes_cmac(key: &[u8], msg: &[u8]) -> [u8; 16] {
    let mut m = <Cmac<Aes128> as Mac>::new_from_slice(key).expect("16-byte key");
    m.update(msg);
    m.finalize().into_bytes().into()
}

/// AES-128-CBC decrypt, zero IV, no padding (spec 6.2 step 2).
fn aes_cbc_decrypt_zero_iv(key: &[u8; 16], data: &[u8]) -> Vec<u8> {
    let cipher = Aes128::new(GenericArray::from_slice(key));
    let mut prev = [0u8; 16];
    let mut out = Vec::with_capacity(data.len());
    for block in data.chunks_exact(16) {
        let mut b = GenericArray::clone_from_slice(block);
        cipher.decrypt_block(&mut b);
        xor_in_place(&mut b, &prev);
        out.extend_from_slice(&b);
        prev.copy_from_slice(block);
    }
    out
}

/// EMSA-PSS encoding: SHA-1, MGF1-SHA-1, salt length 0, emLen 128 (spec 5.1).
fn pss_encode(t: &[u8]) -> [u8; RSA_LEN] {
    const DB_LEN: usize = RSA_LEN - 20 - 1; // 107
    let m_hash = sha1(&[t]);
    let h = sha1(&[&[0u8; 8], &m_hash]);
    let mut db = vec![0u8; DB_LEN];
    db[DB_LEN - 1] = 0x01;
    xor_in_place(&mut db, &mgf1(&h, DB_LEN));
    db[0] &= 0x7f;
    let mut em = [0u8; RSA_LEN];
    em[..DB_LEN].copy_from_slice(&db);
    em[DB_LEN..DB_LEN + 20].copy_from_slice(&h);
    em[RSA_LEN - 1] = 0xbc;
    em
}

/// OAEP unmasking (spec 6.2 step 1). Returns the session key (last 16 bytes of DB)
/// and whether the standard OAEP structure (empty label) checked out. The
/// structure check is only for logging; it never rejects the message.
fn oaep_unmask(em: &[u8; RSA_LEN]) -> ([u8; 16], bool) {
    let mut seed = em[1..21].to_vec();
    let masked_db = &em[21..];
    xor_in_place(&mut seed, &mgf1(masked_db, 20));
    let mut db = masked_db.to_vec();
    let mask = mgf1(&seed, db.len());
    xor_in_place(&mut db, &mask);
    let mut key = [0u8; 16];
    key.copy_from_slice(&db[db.len() - 16..]);
    let structure_ok = em[0] == 0
        && db[..20] == sha1(&[b""])
        && db[20..90].iter().all(|&b| b == 0)
        && db[90] == 0x01;
    (key, structure_ok)
}

// ---------------------------------------------------------------- messages

#[derive(Debug, Error)]
pub enum MtpzError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error("could not get random bytes: {0}")]
    Random(String),
    #[error("device does not advertise microsoft.com/MTPZ")]
    NotMtpz,
    #[error("bad handshake response: {0}")]
    BadResponse(&'static str),
    #[error("device did not echo the host random correctly")]
    RandomMismatch,
}

/// Message 1, the application-certificate message (spec section 5).
pub fn build_message1(keys: &MtpzKeys, host_random: &[u8; 16]) -> Vec<u8> {
    let mut m = vec![0x02, 0x01, 0x01];
    m.extend_from_slice(&(keys.certificate.len() as u32).to_be_bytes());
    m.extend_from_slice(&keys.certificate);
    m.extend_from_slice(&16u16.to_be_bytes());
    m.extend_from_slice(host_random);
    let sig = keys.rsa_private(&pss_encode(&m[2..]));
    m.extend_from_slice(&[0x01, 0x00, 0x80]);
    m.extend_from_slice(&sig);
    m
}

struct Cursor<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize, what: &'static str) -> Result<&'a [u8], MtpzError> {
        let s = self.buf.get(self.pos..self.pos + n).ok_or(MtpzError::BadResponse(what))?;
        self.pos += n;
        Ok(s)
    }
    fn be16(&mut self, what: &'static str) -> Result<usize, MtpzError> {
        Ok(u16::from_be_bytes(self.take(2, what)?.try_into().unwrap()) as usize)
    }
    fn be32(&mut self, what: &'static str) -> Result<usize, MtpzError> {
        Ok(u32::from_be_bytes(self.take(4, what)?.try_into().unwrap()) as usize)
    }
    /// A length field followed by that many bytes.
    fn blob16(&mut self, what: &'static str) -> Result<&'a [u8], MtpzError> {
        let n = self.be16(what)?;
        self.take(n, what)
    }
}

/// What the host keeps from Message 2.
pub struct Message2 {
    /// The MAC seed `S` (at least 20 bytes).
    pub mac_seed: Vec<u8>,
    /// Whether the OAEP structure matched the standard empty-label layout.
    pub oaep_standard: bool,
}

/// Processes Message 2, the device's handshake response (spec section 6).
pub fn process_message2(
    keys: &MtpzKeys,
    msg: &[u8],
    host_random: &[u8; 16],
) -> Result<Message2, MtpzError> {
    let mut c = Cursor { buf: msg, pos: 0 };
    if c.take(2, "marker")? != [0x02, 0x02] {
        return Err(MtpzError::BadResponse("message 2 marker"));
    }
    if c.be16("wrapped key length")? != RSA_LEN {
        return Err(MtpzError::BadResponse("wrapped key length is not 128"));
    }
    let wrapped = c.take(RSA_LEN, "wrapped key")?;
    let payload_len = c.be32("payload length")?;
    if payload_len == 0 || payload_len % 16 != 0 {
        return Err(MtpzError::BadResponse("payload length is not a multiple of 16"));
    }
    let encrypted = c.take(payload_len, "payload")?;

    let (session_key, oaep_standard) = oaep_unmask(&keys.rsa_private(wrapped));
    let p = aes_cbc_decrypt_zero_iv(&session_key, encrypted);

    let mut c = Cursor { buf: &p, pos: 0 };
    c.take(1, "payload marker")?;
    let cert_len = c.be32("device cert length")?;
    c.take(cert_len, "device certs")?;
    if c.blob16("host random")? != host_random {
        return Err(MtpzError::RandomMismatch);
    }
    c.blob16("device random")?;
    c.take(1, "signature marker")?;
    c.blob16("device signature")?;
    c.take(1, "mac marker")?;
    let mac_seed = c.blob16("mac seed")?;
    if mac_seed.len() < 20 {
        return Err(MtpzError::BadResponse("mac seed shorter than 20 bytes"));
    }
    Ok(Message2 { mac_seed: mac_seed.to_vec(), oaep_standard })
}

/// Message 3, the confirmation (spec section 7).
pub fn build_message3(mac_seed: &[u8]) -> Vec<u8> {
    let mut block = [0u8; 16];
    block[15] = 0x01;
    let mut m = vec![0x02, 0x03, 0x00, 0x10];
    m.extend_from_slice(&aes_cmac(&mac_seed[..16], &block));
    m
}

/// Parameters for EnableTrustedFilesOperations (spec section 8).
pub fn enable_params(mac_seed: &[u8]) -> [u32; 4] {
    let t = aes_cmac(&mac_seed[..16], &mac_seed[16..20]);
    std::array::from_fn(|i| u32::from_be_bytes(t[4 * i..4 * i + 4].try_into().unwrap()))
}

/// True if a DeviceInfo VendorExtensionDesc lists `microsoft.com/MTPZ` (spec 2.1).
pub fn advertises_mtpz(vendor_extension_desc: &str) -> bool {
    vendor_extension_desc
        .split(';')
        .filter_map(|e| e.split(':').next())
        .any(|name| name.trim() == "microsoft.com/MTPZ")
}

/// Runs the full handshake (spec section 4) on an open session.
pub fn authenticate(t: &mut Transport, keys: &MtpzKeys) -> Result<Message2, MtpzError> {
    // Spec uncertainty 4: whether the device needs or checks this property.
    // Treat a refusal as non-fatal.
    if let Err(e) = t.transact(
        op::SET_DEVICE_PROP_VALUE,
        &[SESSION_INITIATOR_INFO],
        Some(&dataset::encode_string(INITIATOR_STRING)),
    ) {
        if std::env::var_os("SQUIRT_TRACE").is_some() {
            eprintln!("note: Session Initiator Info refused ({e}); continuing");
        }
    }
    t.transact(op::END_TRUSTED_APP_SESSION, &[], None)?;

    let mut host_random = [0u8; 16];
    getrandom::fill(&mut host_random).map_err(|e| MtpzError::Random(e.to_string()))?;

    t.transact(op::WMDRMPD_SEND_APP_REQUEST, &[], Some(&build_message1(keys, &host_random)))?;
    let reply = t.transact(op::WMDRMPD_GET_APP_RESPONSE, &[], None)?;
    let msg2 = reply.data.ok_or(MtpzError::BadResponse("no data in response"))?;
    let m2 = process_message2(keys, &msg2, &host_random)?;

    t.transact(op::WMDRMPD_SEND_APP_REQUEST, &[], Some(&build_message3(&m2.mac_seed)))?;
    t.transact(op::ENABLE_TRUSTED_FILES_OPERATIONS, &enable_params(&m2.mac_seed), None)?;
    Ok(m2)
}

// ---------------------------------------------------------------- tests

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::BlockEncrypt;

    fn h(s: &str) -> Vec<u8> {
        hex::decode(s).unwrap()
    }

    // Throwaway RSA-1024 key made with `openssl genrsa` for tests only. Not MTPZ key material.
    const TEST_N: &str = "c493f0bf02e24369afca301ddd5c31370100993251316e41b00e3fe1c632e8ac4c95a009b59ccbbab9873783e19afca6cbcc1b2b724d6159605963a0572a1e285950e65985929582e863999d416863931f853b8ae7d4f5a30b00d94f2ba2d5473af0a10d55add088d8a5552e45598fdbb87ea605b9692a34ef4430ac234c591b";
    const TEST_D: &str = "68e5f92634c7c0e978e10fe41fe7be6b77a1761e9fad1d04f889e214fccad5d67bd7ac5b020af5bc629c814762f06dba242e7aa128919a4d3359000ce3f699b846978de051eecc9ea18bcc8f1ff3cd309c402070dfcac2bcda92debd82a1f22561ee927280c6d2ce311ec6c70ebc429e7efc4344cf4b58fc9321e4a19c6b07e1";

    fn fake_chain() -> Vec<u8> {
        (0..629u32).map(|i| i as u8).collect()
    }

    fn test_keys() -> MtpzKeys {
        let file = format!(
            "010001\r\n{}\r\n{TEST_N}\r\n{TEST_D}\r\n{}\r\n",
            "00".repeat(16),
            hex::encode_upper(fake_chain())
        );
        MtpzKeys::parse(&file).unwrap()
    }

    fn rsa_public(x: &[u8]) -> [u8; RSA_LEN] {
        let n = BigUint::from_bytes_be(&h(TEST_N));
        i2osp(&BigUint::from_bytes_be(x).modpow(&BigUint::from(65537u32), &n))
    }

    // --- key file

    #[test]
    fn key_file_parses_and_checks_modulus() {
        let k = test_keys();
        assert_eq!(k.public_exponent, vec![1, 0, 1]);
        assert_eq!(k.certificate.len(), 629);
        assert!(matches!(MtpzKeys::parse("00\n11\n"), Err(KeyError::LineCount(2))));
        assert!(matches!(MtpzKeys::parse("01\n00\nZZ\n00\n00"), Err(KeyError::Hex { line: 3, field: "modulus" })));
        assert!(matches!(MtpzKeys::parse("01\n00\nAABB\n00\n00"), Err(KeyError::ModulusSize(2))));
    }

    // --- spec 10.1 / 10.2 primitive vectors

    #[test]
    fn primitive_vectors() {
        assert_eq!(sha1(&[b"abc"]).to_vec(), h("a9993e364706816aba3e25717850c26c9cd0d89d"));
        let k = h("2b7e151628aed2a6abf7158809cf4f3c");
        assert_eq!(aes_cmac(&k, b"").to_vec(), h("bb1d6929e95937287fa37d129b756746"));
        assert_eq!(
            aes_cmac(&k, &h("6bc1bee22e409f96e93d7e117393172a")).to_vec(),
            h("070a16b46b4d4144f79bdd9dd04a287c")
        );
        assert_eq!(mgf1(b"abc", 107), h("a03eb8ac3e6ad809805ec489ce85f2cefe8d00c69595e19565aec43db343f6e8c11e1224a7617f6f8d54d36ca26d2a23f0224a86d02eccd714a4cff5e64c0a21d72629cc0716386a9d168fccd8fedb83a2936c2ed7aaa104f502b65560480a7296d6011d8ded2246892bcb"));
        let key: [u8; 16] = std::array::from_fn(|i| i as u8);
        let pt: Vec<u8> = (0..32).collect();
        let ct = h("0a940bb5416ef045f1c39458c653ea5a3cf456b4ca488aa383c79c98b34797cb");
        assert_eq!(aes_cbc_decrypt_zero_iv(&key, &ct), pt);
    }

    // --- spec 10.3

    #[test]
    fn pss_vector() {
        let r_h: [u8; 16] = std::array::from_fn(|i| i as u8);
        let mut t = vec![0x01, 0x00, 0x00, 0x02, 0x75];
        t.extend(fake_chain());
        t.extend([0x00, 0x10]);
        t.extend(r_h);
        assert_eq!(t.len(), 652);
        assert_eq!(pss_encode(&t).to_vec(), h("6851f14e28c87bacc2493b9923f3d47b439bcdbfcd6e7040ef65bd4bfb0c3bd3827c1ae95b5c1ee232e7104d3f2ca5d27adc94d81778b7001c95d96c370ff2a69a76a6078df1971b86169a9395a988421a868791a31ebfaff88caae6be7a6c4f648c3575aaa55c417d685074987776b6d6a2590a1ceb9b1dc8b93064d89ef1bc"));
    }

    // --- spec 10.4

    #[test]
    fn oaep_vector() {
        let em: [u8; 128] = h("00701850a18db9d1f69bbb79c77603f6ddae3fcc019d099602de3ca53f8e8de34a2111526ba05b6e5c6397d1c1dbefc476bcaf21ab02161aa67558b1c530ff52c17e7f6e1ab40851c480acc98f56732b39a2085efdf18ddcd840f4ef8dd446990211d087593c2a6bc3dd15e8acc649a66bf7efdd37a6380841c6849309194ac8").try_into().unwrap();
        let (k, ok) = oaep_unmask(&em);
        assert_eq!(k.to_vec(), h("101112131415161718191a1b1c1d1e1f"));
        assert!(ok);
    }

    // --- spec 10.5

    fn test_seed() -> Vec<u8> {
        (0..32).map(|i| 0xa0 + i).collect()
    }

    #[test]
    fn confirmation_and_enable_params() {
        let s = test_seed();
        assert_eq!(build_message3(&s), h("020300102ae71a77ea97545d740f70471bae9e43"));
        assert_eq!(enable_params(&s), [0x792d4e1f, 0xafa730cc, 0x453d422a, 0xa50ed4f6]);
    }

    #[test]
    fn detects_mtpz_extension() {
        let zune_hd = "microsoft.com: 1.0; microsoft.com/WMDRMPD: 10.1; microsoft.com/WMPPD: 11.1; microsoft.com/AAVT: 1.0; microsoft.com/WMDRMND: 1.0; microsoft.com/MTPZ: 1.0; audible.com: 1.0;";
        assert!(advertises_mtpz(zune_hd));
        assert!(!advertises_mtpz("microsoft.com: 1.0; android.com: 1.0;"));
    }

    // --- spec 10.6: end to end against a simulated device

    fn cbc_encrypt_zero_iv(key: &[u8; 16], data: &[u8]) -> Vec<u8> {
        let c = Aes128::new(GenericArray::from_slice(key));
        let mut prev = [0u8; 16];
        let mut out = Vec::new();
        for block in data.chunks_exact(16) {
            let mut b = GenericArray::clone_from_slice(block);
            xor_in_place(&mut b, &prev);
            c.encrypt_block(&mut b);
            prev.copy_from_slice(&b);
            out.extend_from_slice(&b);
        }
        out
    }

    fn device_message2(msg1: &[u8], session_key: [u8; 16], seed: &[u8]) -> Vec<u8> {
        // Check the host signature over offsets 2..654.
        assert_eq!(msg1.len(), 785);
        assert_eq!(&msg1[654..657], &[0x01, 0x00, 0x80]);
        assert_eq!(rsa_public(&msg1[657..]), pss_encode(&msg1[2..654]), "bad PSS signature");
        let host_random = &msg1[638..654];

        // OAEP-wrap the session key, empty label.
        let mut db = sha1(&[b""]).to_vec();
        db.extend([0u8; 70]);
        db.push(0x01);
        db.extend(session_key);
        let oaep_seed = [0x40u8; 20];
        xor_in_place(&mut db, &mgf1(&oaep_seed, 107));
        let mut masked_seed = oaep_seed.to_vec();
        xor_in_place(&mut masked_seed, &mgf1(&db, 20));
        let mut em = vec![0u8];
        em.extend(masked_seed);
        em.extend(db);
        let wrapped = rsa_public(&em);

        // Payload.
        let mut p = vec![0x01];
        p.extend(618u32.to_be_bytes());
        p.extend([0x77; 618]);
        p.extend([0x00, 0x10]);
        p.extend(host_random);
        p.extend([0x00, 0x10]);
        p.extend([0x55; 16]);
        p.extend([0x01, 0x00, 0x80]);
        p.extend([0x66; 128]);
        p.extend([0x01, 0x00, 0x20]);
        p.extend(seed);
        p.resize(832, 0);

        let mut m = vec![0x02, 0x02, 0x00, 0x80];
        m.extend(wrapped);
        m.extend(832u32.to_be_bytes());
        m.extend(cbc_encrypt_zero_iv(&session_key, &p));
        assert_eq!(m.len(), 968);
        m
    }

    #[test]
    fn end_to_end_simulated_device() {
        let keys = test_keys();
        let host_random = [0x3cu8; 16];
        let msg1 = build_message1(&keys, &host_random);
        let msg2 = device_message2(&msg1, std::array::from_fn(|i| 0x10 + i as u8), &test_seed());

        let m2 = process_message2(&keys, &msg2, &host_random).unwrap();
        assert!(m2.oaep_standard);
        assert_eq!(m2.mac_seed, test_seed());
        assert_eq!(enable_params(&m2.mac_seed), [0x792d4e1f, 0xafa730cc, 0x453d422a, 0xa50ed4f6]);

        // A wrong echoed random must be rejected.
        assert!(matches!(
            process_message2(&keys, &msg2, &[0u8; 16]),
            Err(MtpzError::RandomMismatch)
        ));
        // Truncated responses must be rejected, not panic.
        for len in [0, 3, 100, 500, 967] {
            assert!(process_message2(&keys, &msg2[..len], &host_random).is_err());
        }
    }
}
