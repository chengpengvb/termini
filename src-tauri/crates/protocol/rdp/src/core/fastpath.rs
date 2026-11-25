use crate::codec::bulk;
use crate::codec::bulk::BulkContext;
use crate::core::crypt::RdpSecurity;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

pub const UPDATE_TYPE_ORDERS: u8 = 0x0;
pub const UPDATE_TYPE_BITMAP: u8 = 0x1;
pub const UPDATE_TYPE_PALETTE: u8 = 0x2;
pub const UPDATE_TYPE_SYNCHRONIZE: u8 = 0x3;
pub const UPDATE_TYPE_SURFCMDS: u8 = 0x4;
pub const UPDATE_TYPE_PTR_NULL: u8 = 0x5;
pub const UPDATE_TYPE_PTR_DEFAULT: u8 = 0x6;
pub const UPDATE_TYPE_PTR_POSITION: u8 = 0x8;
pub const UPDATE_TYPE_COLOR: u8 = 0x9;
pub const UPDATE_TYPE_CACHED: u8 = 0xA;
pub const UPDATE_TYPE_POINTER: u8 = 0xB;
pub const UPDATE_TYPE_LARGE_POINTER: u8 = 0xC;

const FRAGMENT_SINGLE: u8 = 0;
const FRAGMENT_LAST: u8 = 1;
const FRAGMENT_FIRST: u8 = 2;
const FRAGMENT_NEXT: u8 = 3;

const COMPRESSION_USED: u8 = 2;

pub const INPUT_EVENT_SCANCODE: u8 = 0;
pub const INPUT_EVENT_MOUSE: u8 = 1;
pub const INPUT_EVENT_MOUSEX: u8 = 2;
pub const INPUT_EVENT_SYNC: u8 = 3;
pub const INPUT_EVENT_UNICODE: u8 = 4;
pub const INPUT_EVENT_MOUSE_RELATIVE: u8 = 5;

const INPUT_MAX_INLINE_EVENTS: usize = 15;

pub struct FastPathUpdate {
    pub update_type: u8,
    pub data: Vec<u8>,
}

#[derive(Default)]
pub struct FastPathAssembler {
    buffer: Vec<u8>,
    max_request_size: u32,
    fragment_type: Option<u8>,
}

impl FastPathAssembler {
    pub fn set_max_request_size(&mut self, max_request_size: u32) {
        self.max_request_size = max_request_size;
    }

    pub fn push(&mut self, payload: &[u8], bulk: &mut BulkContext) -> RdpResult<Vec<FastPathUpdate>> {
        let mut cursor = ReadCursor::new(payload);
        let mut updates = Vec::new();
        while cursor.remaining() >= 3 {
            let header = cursor.read_u8()?;
            let update_type = header & 0x0F;
            let fragmentation = (header >> 4) & 0x03;
            let compression = (header >> 6) & 0x03;
            let compression_flags = match compression {
                COMPRESSION_USED => cursor.read_u8()?,
                _ => 0,
            };
            let size = cursor.read_u16_le()? as usize;
            let data = bulk::decompress(bulk, cursor.read_slice(size)?, compression_flags)?;

            match fragmentation {
                FRAGMENT_SINGLE => {
                    if self.fragment_type.is_some() {
                        return Err(RdpProtocolError::protocol("fastpath received a single-fragment update while reassembly is in progress".to_string()));
                    }
                    updates.push(FastPathUpdate { update_type, data: data.into_owned() });
                }
                FRAGMENT_FIRST => {
                    if self.fragment_type.is_some() {
                        return Err(RdpProtocolError::protocol("fastpath received a first fragment while reassembly is in progress".to_string()));
                    }
                    self.fragment_type = Some(update_type);
                    self.buffer = data.into_owned();
                    self.ensure_capacity()?;
                }
                FRAGMENT_NEXT => {
                    if self.fragment_type != Some(update_type) {
                        return Err(RdpProtocolError::protocol("fastpath fragment type mismatch".to_string()));
                    }
                    self.buffer.extend_from_slice(&data);
                    self.ensure_capacity()?;
                }
                FRAGMENT_LAST => {
                    if self.fragment_type != Some(update_type) {
                        return Err(RdpProtocolError::protocol("fastpath fragment type mismatch".to_string()));
                    }
                    self.buffer.extend_from_slice(&data);
                    self.ensure_capacity()?;
                    updates.push(FastPathUpdate { update_type, data: std::mem::take(&mut self.buffer) });
                    self.fragment_type = None;
                }
                _ => return Err(RdpProtocolError::protocol(format!("unexpected fastpath fragmentation flags: {}", fragmentation))),
            }
        }
        Ok(updates)
    }

    fn ensure_capacity(&self) -> RdpResult<()> {
        if self.buffer.len() as u64 > u64::from(self.max_request_size) {
            return Err(RdpProtocolError::protocol(format!("fastpath reassembled length {} exceeds negotiated limit {}", self.buffer.len(), self.max_request_size)));
        }
        Ok(())
    }
}

pub fn build_input_pdus(events: &[Vec<u8>], security: &mut RdpSecurity) -> RdpResult<Vec<Vec<u8>>> {
    if events.is_empty() {
        return Err(RdpProtocolError::protocol("fastpath input event list is empty".to_string()));
    }
    let mut pdus = Vec::new();
    for batch in events.chunks(INPUT_MAX_INLINE_EVENTS) {
        let mut body = Vec::new();
        for event in batch {
            body.extend_from_slice(event);
        }
        let (encryption_flags, body) = security.wrap_fastpath(&body)?;
        let total = 3 + body.len();
        if total > 0x7FFF {
            return Err(RdpProtocolError::encoding(format!("fastpath input PDU too long: {}", total)));
        }
        let mut cursor = WriteCursor::with_capacity(total);
        cursor.write_u8((encryption_flags << 6) | ((batch.len() as u8) << 2));
        cursor.write_u16_be((total as u16) | 0x8000);
        cursor.write_slice(&body);
        pdus.push(cursor.into_vec());
    }
    Ok(pdus)
}
