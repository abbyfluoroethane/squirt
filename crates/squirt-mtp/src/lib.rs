//! Pure-Rust MTP + MTPZ for Microsoft Zune devices.
//!
//! Layers, bottom to top:
//! - [`devices`]: find Zunes on the USB bus.
//! - [`container`]: PTP/MTP container encoding (the wire format).
//! - [`mtpz`]: the MTPZ trust handshake. It needs a user-supplied key file.
//!
//! Still to come: the bulk transport over nusb, sessions, object transfer and
//! album/artwork objects.

pub mod container;
pub mod dataset;
pub mod device;
pub mod devices;
pub mod mtpz;
pub mod transport;
