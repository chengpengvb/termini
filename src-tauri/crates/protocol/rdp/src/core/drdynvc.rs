use std::borrow::Cow;
use crate::core::disp;
use crate::core::rdpgfx;
use crate::codec::zgfx::{self, ZgfxContext};
use crate::core::channel::CHANNEL_CHUNK_LENGTH;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

const CMD_CREATE: u8 = 0x01;
const CMD_DATA_FIRST: u8 = 0x02;
const CMD_DATA: u8 = 0x03;
const CMD_CLOSE: u8 = 0x04;
const CMD_CAPABILITIES: u8 = 0x05;
const CMD_DATA_FIRST_COMPRESSED: u8 = 0x06;
const CMD_DATA_COMPRESSED: u8 = 0x07;

const CAPABILITIES_VERSION_2: u16 = 0x0002;
const CAPABILITIES_VERSION_3: u16 = 0x0003;
const PRIORITY_CHARGE_LENGTH: usize = 8;

const CREATION_STATUS_OK: u32 = 0x0000_0000;
const CREATION_STATUS_UNSUCCESSFUL: u32 = 0xC000_0001;
const CREATION_STATUS_NOT_FOUND: u32 = 0xC000_0225;

const LISTENERS: [&str; 2] = [disp::CHANNEL_NAME, rdpgfx::CHANNEL_NAME];

struct Fragment {
    expected: usize,
    buffer: Vec<u8>,
}

struct DynamicChannel {
    id: u32,
    name: &'static str,
    fragment: Option<Fragment>,
    decompressor: Option<ZgfxContext>,
}

pub struct DynamicMessage {
    pub name: &'static str,
    pub payload: Vec<u8>,
}

#[derive(Default)]
pub struct DynamicOutcome {
    pub responses: Vec<Vec<u8>>,
    pub created: Option<&'static str>,
    pub message: Option<DynamicMessage>,
}

#[derive(Default)]
pub struct DynamicChannels {
    channels: Vec<DynamicChannel>,
}

impl DynamicChannels {
    pub fn channel_id(&self, name: &str) -> Option<u32> {
        self.channels.iter().find(|channel| channel.name == name).map(|channel| channel.id)
    }

    fn channel_mut(&mut self, id: u32) -> Option<&mut DynamicChannel> {
        self.channels.iter_mut().find(|channel| channel.id == id)
    }
}

pub fn process(state: &mut DynamicChannels, message: &[u8]) -> RdpResult<DynamicOutcome> {
    let mut cursor = ReadCursor::new(message);
    let value = cursor.read_u8()?;
    let command = value >> 4;
    let length_bytes = (value & 0x0C) >> 2;
    let channel_bytes = value & 0x03;
    match command {
        CMD_CAPABILITIES => Ok(DynamicOutcome { responses: vec![read_capabilities(&mut cursor)?], ..Default::default() }),
        CMD_CREATE => read_create(state, &mut cursor, channel_bytes),
        CMD_DATA_FIRST | CMD_DATA_FIRST_COMPRESSED => {
            let id = read_variable_uint(&mut cursor, channel_bytes)?;
            let expected = read_variable_uint(&mut cursor, length_bytes)? as usize;
            read_data(state, id, cursor.rest(), Some(expected), command == CMD_DATA_FIRST_COMPRESSED)
        }
        CMD_DATA | CMD_DATA_COMPRESSED => {
            let id = read_variable_uint(&mut cursor, channel_bytes)?;
            read_data(state, id, cursor.rest(), None, command == CMD_DATA_COMPRESSED)
        }
        CMD_CLOSE => read_close(state, &mut cursor, channel_bytes),
        other => Err(RdpProtocolError::unsupported(format!("drdynvc command 0x{:02x} not implemented", other))),
    }
}

pub fn build_data(channel_id: u32, payload: &[u8]) -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(payload.len() + 5);
    cursor.write_u8(0);
    let channel_bytes = write_variable_uint(&mut cursor, channel_id);
    cursor.patch_u8(0, (CMD_DATA << 4) | channel_bytes)?;
    if cursor.len() + payload.len() > CHANNEL_CHUNK_LENGTH as usize {
        return Err(RdpProtocolError::encoding(format!("drdynvc data exceeds single-chunk length: {}", payload.len())));
    }
    cursor.write_slice(payload);
    Ok(cursor.into_vec())
}

fn read_capabilities(cursor: &mut ReadCursor<'_>) -> RdpResult<Vec<u8>> {
    cursor.skip(1)?;
    let version = cursor.read_u16_le()?;
    if version == CAPABILITIES_VERSION_2 || version == CAPABILITIES_VERSION_3 {
        cursor.skip(PRIORITY_CHARGE_LENGTH)?;
    }
    let mut response = WriteCursor::with_capacity(4);
    response.write_u8(CMD_CAPABILITIES << 4);
    response.write_u8(0);
    response.write_u16_le(version);
    Ok(response.into_vec())
}

fn read_create(state: &mut DynamicChannels, cursor: &mut ReadCursor<'_>, channel_bytes: u8) -> RdpResult<DynamicOutcome> {
    let id_bytes = cursor.read_vec(variable_uint_length(channel_bytes))?;
    let id = decode_variable_uint(&id_bytes);
    let name = read_channel_name(cursor)?;
    let mut created = None;
    let status = match LISTENERS.into_iter().find(|listener| *listener == name) {
        None => CREATION_STATUS_NOT_FOUND,
        Some(listener) => match state.channel_mut(id).is_some() {
            true => CREATION_STATUS_UNSUCCESSFUL,
            false => {
                state.channels.push(DynamicChannel { id, name: listener, fragment: None, decompressor: None });
                created = Some(listener);
                CREATION_STATUS_OK
            }
        },
    };
    let mut response = WriteCursor::with_capacity(id_bytes.len() + 5);
    response.write_u8((CMD_CREATE << 4) | channel_bytes);
    response.write_slice(&id_bytes);
    response.write_u32_le(status);
    Ok(DynamicOutcome { responses: vec![response.into_vec()], created, ..Default::default() })
}

fn read_close(state: &mut DynamicChannels, cursor: &mut ReadCursor<'_>, channel_bytes: u8) -> RdpResult<DynamicOutcome> {
    let id = read_variable_uint(cursor, channel_bytes)?;
    if state.channel_mut(id).is_none() {
        return Ok(DynamicOutcome::default());
    }
    state.channels.retain(|channel| channel.id != id);
    let mut response = WriteCursor::with_capacity(5);
    response.write_u8((CMD_CLOSE << 4) | 0x02);
    response.write_u32_le(id);
    Ok(DynamicOutcome { responses: vec![response.into_vec()], ..Default::default() })
}

fn read_data(state: &mut DynamicChannels, id: u32, payload: &[u8], expected: Option<usize>, compressed: bool) -> RdpResult<DynamicOutcome> {
    let Some(channel) = state.channel_mut(id) else {
        return Ok(DynamicOutcome::default());
    };
    let data = match compressed {
        true => Cow::Owned(zgfx::decompress(channel.decompressor.get_or_insert_with(ZgfxContext::default), payload)?),
        false => Cow::Borrowed(payload),
    };
    if let Some(expected) = expected {
        channel.fragment = Some(Fragment { expected, buffer: Vec::with_capacity(expected) });
    }
    let Some(mut fragment) = channel.fragment.take() else {
        return Ok(DynamicOutcome { message: Some(DynamicMessage { name: channel.name, payload: data.into_owned() }), ..Default::default() });
    };
    if fragment.buffer.len() + data.len() > fragment.expected {
        return Err(RdpProtocolError::protocol(format!("drdynvc fragment data exceeds declared length: {}", fragment.expected)));
    }
    fragment.buffer.extend_from_slice(&data);
    if fragment.buffer.len() < fragment.expected {
        channel.fragment = Some(fragment);
        return Ok(DynamicOutcome::default());
    }
    Ok(DynamicOutcome { message: Some(DynamicMessage { name: channel.name, payload: fragment.buffer }), ..Default::default() })
}

fn read_channel_name(cursor: &mut ReadCursor<'_>) -> RdpResult<String> {
    let mut name = Vec::new();
    loop {
        let byte = cursor.read_u8()?;
        if byte == 0 {
            return String::from_utf8(name).map_err(|error| RdpProtocolError::encoding(format!("invalid drdynvc channel name: {}", error)));
        }
        name.push(byte);
    }
}

fn variable_uint_length(channel_bytes: u8) -> usize {
    match channel_bytes {
        0 => 1,
        1 => 2,
        _ => 4,
    }
}

fn decode_variable_uint(bytes: &[u8]) -> u32 {
    bytes.iter().rev().fold(0u32, |value, byte| (value << 8) | u32::from(*byte))
}

fn read_variable_uint(cursor: &mut ReadCursor<'_>, channel_bytes: u8) -> RdpResult<u32> {
    Ok(decode_variable_uint(cursor.read_slice(variable_uint_length(channel_bytes))?))
}

fn write_variable_uint(cursor: &mut WriteCursor, value: u32) -> u8 {
    match value {
        0x0000_0000..=0x0000_00FF => {
            cursor.write_u8(value as u8);
            0x00
        }
        0x0000_0100..=0x0000_FFFF => {
            cursor.write_u16_le(value as u16);
            0x01
        }
        _ => {
            cursor.write_u32_le(value);
            0x02
        }
    }
}
