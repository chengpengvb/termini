use std::borrow::Cow;
use crate::codec::mppc;
use crate::codec::ncrush;
use crate::codec::xcrush;
use crate::codec::mppc::MppcContext;
use crate::codec::ncrush::NcrushContext;
use crate::codec::xcrush::XcrushContext;
use crate::core::error::{RdpProtocolError, RdpResult};

pub const PACKET_COMPRESSED: u8 = 0x20;
pub const PACKET_AT_FRONT: u8 = 0x40;
pub const PACKET_FLUSHED: u8 = 0x80;

pub const PACKET_COMPR_TYPE_8K: u8 = 0x00;
pub const PACKET_COMPR_TYPE_64K: u8 = 0x01;
pub const PACKET_COMPR_TYPE_RDP6: u8 = 0x02;
pub const PACKET_COMPR_TYPE_RDP61: u8 = 0x03;

const COMPRESSION_FLAGS_MASK: u8 = 0xE0;
const COMPRESSION_TYPE_MASK: u8 = 0x0F;

#[derive(Default)]
pub struct BulkContext {
    mppc: MppcContext,
    ncrush: NcrushContext,
    xcrush: XcrushContext,
}

pub fn decompress<'a>(context: &mut BulkContext, source: &'a [u8], flags: u8) -> RdpResult<Cow<'a, [u8]>> {
    if flags & COMPRESSION_FLAGS_MASK == 0 {
        return Ok(Cow::Borrowed(source));
    }
    match flags & COMPRESSION_TYPE_MASK {
        PACKET_COMPR_TYPE_8K => mppc::decompress(&mut context.mppc, source, flags, mppc::LEVEL_8K),
        PACKET_COMPR_TYPE_64K => mppc::decompress(&mut context.mppc, source, flags, mppc::LEVEL_64K),
        PACKET_COMPR_TYPE_RDP61 => Ok(Cow::Owned(xcrush::decompress(&mut context.xcrush, source, flags)?)),
        PACKET_COMPR_TYPE_RDP6 => Ok(Cow::Owned(ncrush::decompress(&mut context.ncrush, source, flags)?)),
        other => Err(RdpProtocolError::unsupported(format!("server used an unadvertised compression type: 0x{:02x}", other))),
    }
}
