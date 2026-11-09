//! MTP-over-USB transport (PIMA 15740 / MTP 1.1 USB "Still Image" bulk-only
//! protocol). A transaction is: command container -> optional data phase ->
//! response container. Each container ends with a short or zero-length packet.

use crate::container::{Container, ContainerError, ContainerType, RESPONSE_OK};
use nusb::descriptors::TransferType;
use nusb::io::{EndpointRead, EndpointWrite};
use nusb::transfer::{Bulk, Direction, In, Out};
use nusb::MaybeFuture;
use std::io::{Read, Write};
use std::time::Duration;
use thiserror::Error;

const TRANSFER_SIZE: usize = 128 * 1024;
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("usb: {0}")]
    Usb(#[from] nusb::Error),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("device has no usable USB configuration")]
    NoConfiguration,
    #[error("no interface with bulk IN and OUT endpoints")]
    NoBulkInterface,
    #[error("bad container: {0}")]
    Container(#[from] ContainerError),
    #[error("expected {expected:?}, got {got:?} (code {code:#06x})")]
    Unexpected { expected: ContainerType, got: ContainerType, code: u16 },
    #[error("transaction id mismatch: sent {sent}, got {got}")]
    TransactionMismatch { sent: u32, got: u32 },
    #[error("device returned response {0:#06x}")]
    Response(u16),
}

/// The result of one transaction.
#[derive(Debug)]
pub struct Reply {
    pub code: u16,
    pub params: Vec<u32>,
    /// Data phase from device to host, if there was one.
    pub data: Option<Vec<u8>>,
}

pub struct Transport {
    _iface: nusb::Interface,
    out: EndpointWrite<Bulk>,
    inp: EndpointRead<Bulk>,
    next_tid: u32,
}

impl Transport {
    /// Opens the device and claims the first interface that has bulk IN and OUT
    /// endpoints. The Zune may report a vendor class rather than Still Image, so
    /// we match on endpoints, not on class.
    pub fn open(info: &nusb::DeviceInfo) -> Result<Self, TransportError> {
        let dev = info.open().wait()?;
        // macOS (and Linux without a driver) can leave the device unconfigured.
        // If so, select the first configuration ourselves.
        if dev.active_configuration().is_err() {
            let value = dev
                .configurations()
                .next()
                .map(|c| c.configuration_value())
                .ok_or(TransportError::NoConfiguration)?;
            dev.set_configuration(value).wait()?;
        }
        let (iface_num, ep_in, ep_out) = {
            let cfg = dev.active_configuration().map_err(|_| TransportError::NoConfiguration)?;
            cfg.interfaces()
                .find_map(|ifc| {
                    let alt = ifc.alt_settings().next()?;
                    let mut ep_in = None;
                    let mut ep_out = None;
                    for ep in alt.endpoints() {
                        if ep.transfer_type() != TransferType::Bulk {
                            continue;
                        }
                        match ep.direction() {
                            Direction::In => ep_in = Some(ep.address()),
                            Direction::Out => ep_out = Some(ep.address()),
                        }
                    }
                    Some((ifc.interface_number(), ep_in?, ep_out?))
                })
                .ok_or(TransportError::NoBulkInterface)?
        };
        let iface = dev.detach_and_claim_interface(iface_num).wait()?;
        let out = iface
            .endpoint::<Bulk, Out>(ep_out)?
            .writer(TRANSFER_SIZE)
            .with_write_timeout(TIMEOUT);
        let mut inp = iface
            .endpoint::<Bulk, In>(ep_in)?
            .reader(TRANSFER_SIZE)
            .with_read_timeout(Duration::from_millis(300));
        // A previous run may have stopped mid-transaction, leaving the device
        // waiting for us to read its reply. It will not accept new commands
        // until that is read, so drain it.
        let mut drained = 0usize;
        let mut scratch = Vec::new();
        loop {
            scratch.clear();
            let mut rd = inp.until_short_packet();
            match rd.read_to_end(&mut scratch) {
                Ok(_) => {
                    let _ = rd.consume_end();
                    drained += scratch.len();
                    if drained > 64 * 1024 * 1024 {
                        break;
                    }
                }
                Err(_) => break, // Timed out: nothing left.
            }
        }
        if drained > 0 && std::env::var_os("SQUIRT_TRACE").is_some() {
            eprintln!("drained {drained} stale bytes from device");
        }
        inp.set_read_timeout(TIMEOUT);
        Ok(Self { _iface: iface, out, inp, next_tid: 0 })
    }

    fn send(&mut self, c: &Container) -> Result<(), TransportError> {
        self.out.write_all(&c.encode())?;
        self.out.flush_end()?;
        Ok(())
    }

    /// Reads one short-packet-terminated USB transfer.
    fn read_transfer(&mut self, buf: &mut Vec<u8>) -> Result<(), TransportError> {
        let mut rd = self.inp.until_short_packet();
        rd.read_to_end(buf)?;
        rd.consume_end().map_err(|_| std::io::Error::other("missing short packet"))?;
        Ok(())
    }

    /// Reads one container. Some devices (the Zune HD among them) send the
    /// 12-byte header as its own short transfer, before the payload. So keep
    /// reading until the declared length is reached.
    fn recv(&mut self) -> Result<Container, TransportError> {
        let mut buf = Vec::new();
        self.read_transfer(&mut buf)?;
        if buf.len() >= 4 {
            let declared = u32::from_le_bytes(buf[..4].try_into().unwrap()) as usize;
            while buf.len() < declared {
                let before = buf.len();
                self.read_transfer(&mut buf)?;
                if buf.len() == before {
                    break; // A zero-length transfer with nothing more to read.
                }
            }
        }
        Ok(Container::decode(&buf)?)
    }

    /// Runs one transaction. `data_out` is sent as a host-to-device data phase.
    /// A device-to-host data phase, if the device sends one, is returned in
    /// [`Reply::data`]. A non-OK response code becomes an error.
    pub fn transact(
        &mut self,
        code: u16,
        params: &[u32],
        data_out: Option<&[u8]>,
    ) -> Result<Reply, TransportError> {
        let tid = self.next_tid;
        self.next_tid = self.next_tid.wrapping_add(1);
        let trace = std::env::var_os("SQUIRT_TRACE").is_some();
        if trace {
            eprintln!("-> op {code:#06x} tid {tid} params {params:x?} data_out {:?}", data_out.map(<[u8]>::len));
        }

        self.send(&Container::command(code, tid, params)?)?;
        if let Some(data) = data_out {
            let bytes = Container {
                kind: ContainerType::Data,
                code,
                transaction_id: tid,
                payload: data.to_vec(),
            }
            .encode();
            // The Zune HD rejects (0x2002) a data container sent as one
            // transfer. It wants the 12-byte header as its own short transfer,
            // then the payload, mirroring how it sends data to us.
            self.out.write_all(&bytes[..crate::container::HEADER_LEN])?;
            self.out.flush_end()?;
            self.out.write_all(&bytes[crate::container::HEADER_LEN..])?;
            self.out.flush_end()?;
        }

        let mut data_in = None;
        let mut c = self.recv()?;
        if c.kind == ContainerType::Data {
            data_in = Some(std::mem::take(&mut c.payload));
            c = self.recv()?;
        }
        if trace {
            eprintln!("<- {:?} code {:#06x} tid {} data_in {:?}", c.kind, c.code, c.transaction_id, data_in.as_ref().map(Vec::len));
        }
        if c.kind != ContainerType::Response {
            return Err(TransportError::Unexpected {
                expected: ContainerType::Response,
                got: c.kind,
                code: c.code,
            });
        }
        if c.transaction_id != tid {
            return Err(TransportError::TransactionMismatch { sent: tid, got: c.transaction_id });
        }
        if c.code != RESPONSE_OK {
            return Err(TransportError::Response(c.code));
        }
        Ok(Reply { code: c.code, params: c.params(), data: data_in })
    }

    /// OpenSession is the one command sent with transaction id 0. Later
    /// transactions count up from 1.
    pub fn open_session(&mut self, session_id: u32) -> Result<(), TransportError> {
        self.next_tid = 0;
        self.transact(crate::container::op::OPEN_SESSION, &[session_id], None)?;
        Ok(())
    }
}
