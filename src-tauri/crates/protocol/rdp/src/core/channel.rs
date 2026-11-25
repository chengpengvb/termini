use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

const CHANNEL_FLAG_FIRST: u32 = 0x0000_0001;
const CHANNEL_FLAG_LAST: u32 = 0x0000_0002;
const CHANNEL_FLAG_SHOW_PROTOCOL: u32 = 0x0000_0010;
const CHANNEL_FLAG_SUSPEND: u32 = 0x0000_0020;
const CHANNEL_FLAG_RESUME: u32 = 0x0000_0040;

pub const DRDYNVC_CHANNEL_NAME: &str = "drdynvc";
pub const CLIPRDR_CHANNEL_NAME: &str = "cliprdr";
pub const RDPDR_CHANNEL_NAME: &str = "rdpdr";
pub const RDPSND_CHANNEL_NAME: &str = "rdpsnd";

pub const CHANNEL_CHUNK_LENGTH: u32 = 1600;

const CHANNEL_NAME_LENGTH: usize = 7;
const CHANNEL_CHUNK_MAX_LENGTH: u32 = 16256;
const CHANNEL_PDU_HEADER_LENGTH: usize = 8;

const CHANNEL_OPTION_INITIALIZED: u32 = 0x8000_0000;
const CHANNEL_OPTION_ENCRYPT_RDP: u32 = 0x4000_0000;
const CHANNEL_OPTION_COMPRESS_RDP: u32 = 0x0080_0000;
const CHANNEL_OPTION_SHOW_PROTOCOL: u32 = 0x0020_0000;

pub struct ChannelDefinition {
    pub name: &'static str,
    pub options: u32,
}

pub const CLIENT_CHANNELS: [ChannelDefinition; 4] = [
    ChannelDefinition { name: DRDYNVC_CHANNEL_NAME, options: CHANNEL_OPTION_INITIALIZED | CHANNEL_OPTION_ENCRYPT_RDP | CHANNEL_OPTION_COMPRESS_RDP },
    ChannelDefinition { name: CLIPRDR_CHANNEL_NAME, options: CHANNEL_OPTION_INITIALIZED | CHANNEL_OPTION_ENCRYPT_RDP | CHANNEL_OPTION_COMPRESS_RDP | CHANNEL_OPTION_SHOW_PROTOCOL },
    ChannelDefinition { name: RDPDR_CHANNEL_NAME, options: CHANNEL_OPTION_INITIALIZED | CHANNEL_OPTION_ENCRYPT_RDP | CHANNEL_OPTION_COMPRESS_RDP },
    ChannelDefinition { name: RDPSND_CHANNEL_NAME, options: CHANNEL_OPTION_INITIALIZED | CHANNEL_OPTION_ENCRYPT_RDP },
];

pub struct ChannelChunk<'a> {
    pub total_length: u32,
    pub flags: u32,
    pub payload: &'a [u8],
}

struct StaticChannel {
    name: &'static str,
    id: u16,
    options: u32,
    buffer: Vec<u8>,
}

pub struct ChannelTable {
    channels: Vec<StaticChannel>,
    chunk_size: u32,
}

impl ChannelTable {
    pub fn new(ids: &[u16]) -> RdpResult<Self> {
        if ids.len() > CLIENT_CHANNELS.len() {
            return Err(RdpProtocolError::protocol(format!("server returned more virtual channels than requested: requested {} returned {}", CLIENT_CHANNELS.len(), ids.len())));
        }
        let channels = CLIENT_CHANNELS.iter().zip(ids).map(|(definition, id)| StaticChannel { name: definition.name, id: *id, options: definition.options, buffer: Vec::new() }).collect();
        Ok(Self { channels, chunk_size: CHANNEL_CHUNK_LENGTH })
    }

    pub fn set_chunk_size(&mut self, advertised: Option<u32>) {
        self.chunk_size = match advertised {
            Some(size) if size > 0 && size <= CHANNEL_CHUNK_MAX_LENGTH => size,
            _ => CHANNEL_CHUNK_LENGTH,
        };
    }

    pub fn chunk_size(&self) -> u32 {
        self.chunk_size
    }

    pub fn channel_name(&self, id: u16) -> Option<&'static str> {
        self.channels.iter().find(|channel| channel.id == id).map(|channel| channel.name)
    }

    pub fn channel_id(&self, name: &str) -> Option<u16> {
        self.channels.iter().find(|channel| channel.name == name).map(|channel| channel.id)
    }

    pub fn reassemble(&mut self, id: u16, chunk: ChannelChunk<'_>) -> Option<Vec<u8>> {
        let channel = self.channels.iter_mut().find(|channel| channel.id == id)?;
        if chunk.flags & (CHANNEL_FLAG_SUSPEND | CHANNEL_FLAG_RESUME) != 0 {
            return None;
        }
        if chunk.flags & CHANNEL_FLAG_FIRST != 0 {
            channel.buffer.clear();
            channel.buffer.reserve(chunk.total_length as usize);
        }
        channel.buffer.extend_from_slice(chunk.payload);
        if chunk.flags & CHANNEL_FLAG_LAST == 0 {
            return None;
        }
        Some(std::mem::take(&mut channel.buffer))
    }

    pub fn split(&self, id: u16, data: &[u8]) -> RdpResult<Vec<Vec<u8>>> {
        let channel = self.channels.iter().find(|channel| channel.id == id).ok_or_else(|| RdpProtocolError::protocol(format!("virtual channel {} not negotiated", id)))?;
        let extra = if channel.options & CHANNEL_OPTION_SHOW_PROTOCOL != 0 { CHANNEL_FLAG_SHOW_PROTOCOL } else { 0 };
        let total = u32::try_from(data.len()).map_err(|_| RdpProtocolError::encoding(format!("virtual channel data too long: {}", data.len())))?;
        let chunk_size = self.chunk_size as usize;
        let mut packets = Vec::new();
        let mut flags = CHANNEL_FLAG_FIRST | extra;
        let mut offset = 0usize;
        while offset < data.len() {
            let length = (data.len() - offset).min(chunk_size);
            if offset + length == data.len() {
                flags |= CHANNEL_FLAG_LAST;
            }
            let mut cursor = WriteCursor::with_capacity(length + CHANNEL_PDU_HEADER_LENGTH);
            cursor.write_u32_le(total);
            cursor.write_u32_le(flags);
            cursor.write_slice(&data[offset..offset + length]);
            packets.push(cursor.into_vec());
            offset += length;
            flags = extra;
        }
        Ok(packets)
    }
}

pub fn write_client_network(cursor: &mut WriteCursor, channels: &[ChannelDefinition]) -> RdpResult<()> {
    cursor.write_u32_le(u32::try_from(channels.len()).map_err(|_| RdpProtocolError::encoding(format!("too many virtual channels: {}", channels.len())))?);
    for channel in channels {
        let padding = (CHANNEL_NAME_LENGTH + 1).checked_sub(channel.name.len()).ok_or_else(|| RdpProtocolError::encoding(format!("virtual channel name too long: {}", channel.name)))?;
        cursor.write_slice(channel.name.as_bytes());
        cursor.write_zeros(padding);
        cursor.write_u32_le(channel.options);
    }
    Ok(())
}

pub fn parse_chunk(data: &[u8]) -> RdpResult<ChannelChunk<'_>> {
    let mut cursor = ReadCursor::new(data);
    let total_length = cursor.read_u32_le()?;
    let flags = cursor.read_u32_le()?;
    Ok(ChannelChunk { total_length, flags, payload: cursor.rest() })
}
