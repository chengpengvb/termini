use crate::core::per;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::channel::{self, ChannelDefinition};
use crate::core::error::{RdpProtocolError, RdpResult};

const T124_02_98_OID: [u8; 6] = [0, 0, 20, 124, 0, 1];
const H221_CS_KEY: &[u8; 4] = b"Duca";
const H221_SC_KEY: &[u8; 4] = b"McDn";

const CS_CORE: u16 = 0xC001;
const CS_SECURITY: u16 = 0xC002;
const CS_NET: u16 = 0xC003;
const CS_CLUSTER: u16 = 0xC004;
const CS_MONITOR: u16 = 0xC005;
const CS_MCS_MSGCHANNEL: u16 = 0xC006;
const CS_MONITOR_EX: u16 = 0xC008;
const CS_MULTITRANSPORT: u16 = 0xC00A;

const SC_CORE: u16 = 0x0C01;
const SC_SECURITY: u16 = 0x0C02;
const SC_NET: u16 = 0x0C03;
const SC_MCS_MSGCHANNEL: u16 = 0x0C04;

const RDP_VERSION_5_PLUS: u32 = 0x0008_0004;
const RNS_UD_COLOR_8BPP: u16 = 0xCA01;
const RNS_UD_SAS_DEL: u16 = 0xAA03;

const RNS_UD_24BPP_SUPPORT: u16 = 0x0001;
const RNS_UD_16BPP_SUPPORT: u16 = 0x0002;
const RNS_UD_15BPP_SUPPORT: u16 = 0x0004;
const RNS_UD_32BPP_SUPPORT: u16 = 0x0008;

const RNS_UD_CS_SUPPORT_ERRINFO_PDU: u16 = 0x0001;
const RNS_UD_CS_WANT_32BPP_SESSION: u16 = 0x0002;
const RNS_UD_CS_SUPPORT_STATUSINFO_PDU: u16 = 0x0004;
const RNS_UD_CS_STRONG_ASYMMETRIC_KEYS: u16 = 0x0008;
const RNS_UD_CS_VALID_CONNECTION_TYPE: u16 = 0x0020;
const RNS_UD_CS_SUPPORT_MONITOR_LAYOUT_PDU: u16 = 0x0040;
const RNS_UD_CS_SUPPORT_NETCHAR_AUTODETECT: u16 = 0x0080;
const RNS_UD_CS_SUPPORT_DYNVC_GFX_PROTOCOL: u16 = 0x0100;
const RNS_UD_CS_SUPPORT_HEARTBEAT_PDU: u16 = 0x0400;

const CONNECTION_TYPE_LAN: u8 = 0x06;

const TRANSPORT_TYPE_UDP_FECR: u32 = 0x0000_0001;

const REDIRECTION_SUPPORTED: u32 = 0x0000_0001;
const REDIRECTED_SESSIONID_FIELD_VALID: u32 = 0x0000_0002;
const REDIRECTION_VERSION4: u32 = 0x0000_0003;

const ENCRYPTION_METHOD_NONE: u32 = 0x0000_0000;
const ENCRYPTION_LEVEL_NONE: u32 = 0x0000_0000;

const CLIENT_BUILD: u32 = 3790;
const CLIENT_PRODUCT_ID: u16 = 1;
const KEYBOARD_TYPE_IBM_ENHANCED: u32 = 4;
const KEYBOARD_FUNCTION_KEYS: u32 = 12;
const MONITOR_PRIMARY: u32 = 0x0000_0001;
const MONITOR_ATTRIBUTE_SIZE: u32 = 20;
const ORIENTATION_LANDSCAPE: u32 = 0;
const PHYSICAL_MM_MIN: u32 = 10;
const PHYSICAL_MM_MAX: u32 = 10000;
const DESKTOP_SCALE_MIN: u32 = 100;
const DESKTOP_SCALE_MAX: u32 = 500;
const STANDARD_DPI: u32 = 96;

#[derive(Clone, Copy)]
pub struct Monitor {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
    pub physical_width: u32,
    pub physical_height: u32,
    pub orientation: u32,
    pub desktop_scale_factor: u32,
    pub device_scale_factor: u32,
}

impl Monitor {
    pub fn from_size(width: u16, height: u16, dpi: u32, primary: bool) -> RdpResult<Self> {
        if dpi == 0 {
            return Err(RdpProtocolError::protocol("monitor DPI is 0".to_string()));
        }
        let desktop_scale_factor = u32::try_from(u64::from(dpi) * 100 / u64::from(STANDARD_DPI)).map_err(|_| RdpProtocolError::protocol(format!("desktopScaleFactor overflow: dpi={}", dpi)))?;
        if desktop_scale_factor < DESKTOP_SCALE_MIN || desktop_scale_factor > DESKTOP_SCALE_MAX {
            return Err(RdpProtocolError::protocol(format!("desktopScaleFactor out of range: {} (dpi={})", desktop_scale_factor, dpi)));
        }
        Ok(Self {
            left: 0,
            top: 0,
            width: u32::from(width),
            height: u32::from(height),
            primary,
            physical_width: physical_mm(u32::from(width), dpi)?,
            physical_height: physical_mm(u32::from(height), dpi)?,
            orientation: ORIENTATION_LANDSCAPE,
            desktop_scale_factor,
            device_scale_factor: 100,
        })
    }
}

fn physical_mm(pixels: u32, dpi: u32) -> RdpResult<u32> {
    let mm = (u64::from(pixels) * 254 + u64::from(dpi) * 5) / (u64::from(dpi) * 10);
    let mm = u32::try_from(mm).map_err(|_| RdpProtocolError::protocol(format!("physical size overflow: {}px @{}dpi", pixels, dpi)))?;
    if mm < PHYSICAL_MM_MIN || mm > PHYSICAL_MM_MAX {
        return Err(RdpProtocolError::protocol(format!("physical size out of range: {} mm ({}px @{}dpi)", mm, pixels, dpi)));
    }
    Ok(mm)
}

pub struct ClientCoreSettings {
    pub desktop_width: u16,
    pub desktop_height: u16,
    pub color_depth: u16,
    pub keyboard_layout: u32,
    pub client_name: String,
    pub selected_protocol: u32,
    pub console_session: bool,
    pub extended_client_data: bool,
    pub encryption_methods: u32,
    pub monitors: Vec<Monitor>,
}

#[derive(Debug, Default, Clone)]
pub struct ServerData {
    pub version: u32,
    pub encryption_method: u32,
    pub encryption_level: u32,
    pub mcs_channel_id: u16,
    pub message_channel_id: u16,
    pub channel_ids: Vec<u16>,
    pub server_random: Vec<u8>,
    pub server_certificate: Vec<u8>,
}

pub fn build_conference_create_request(user_data: &[u8]) -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(user_data.len() + 32);
    per::write_choice(&mut cursor, 0);
    per::write_object_identifier(&mut cursor, &T124_02_98_OID);
    per::write_length(&mut cursor, u16::try_from(user_data.len() + 14).map_err(|_| RdpProtocolError::encoding("GCC user data too long".to_string()))?)?;
    per::write_choice(&mut cursor, 0);
    per::write_selection(&mut cursor, 0x08);
    per::write_numeric_string(&mut cursor, b"1", 1)?;
    per::write_padding(&mut cursor, 1);
    per::write_number_of_sets(&mut cursor, 1);
    per::write_choice(&mut cursor, 0xC0);
    per::write_octet_string(&mut cursor, H221_CS_KEY, 4)?;
    per::write_octet_string(&mut cursor, user_data, 0)?;
    Ok(cursor.into_vec())
}

pub fn parse_conference_create_response(payload: &[u8]) -> RdpResult<ServerData> {
    let mut cursor = ReadCursor::new(payload);
    per::read_choice(&mut cursor)?;
    per::read_object_identifier(&mut cursor, &T124_02_98_OID)?;
    per::read_length(&mut cursor)?;
    per::read_choice(&mut cursor)?;
    per::read_integer16(&mut cursor, 1001)?;
    per::read_integer(&mut cursor)?;
    let result = per::read_enumerated(&mut cursor, 16)?;
    if result != 0 {
        return Err(RdpProtocolError::rejected(format!("GCC conference create rejected: result={}", result)));
    }
    per::read_number_of_sets(&mut cursor)?;
    per::read_choice(&mut cursor)?;
    let key = per::read_octet_string(&mut cursor, 4)?;
    if key != H221_SC_KEY {
        return Err(RdpProtocolError::protocol(format!("unexpected GCC H221 key: {:02x?}", key)));
    }
    let length = per::read_length(&mut cursor)? as usize;
    let blocks = cursor.read_slice(length)?;
    parse_server_data_blocks(blocks)
}

pub fn build_client_data(settings: &ClientCoreSettings, channels: &[ChannelDefinition]) -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(512);
    write_client_core(&mut cursor, settings)?;
    write_client_cluster(&mut cursor, settings)?;
    write_client_security(&mut cursor, settings)?;
    write_client_network(&mut cursor, channels)?;
    write_client_monitor(&mut cursor, settings)?;
    write_client_monitor_ex(&mut cursor, settings)?;
    if settings.extended_client_data {
        write_client_message_channel(&mut cursor)?;
        write_client_multitransport(&mut cursor)?;
    }
    Ok(cursor.into_vec())
}

fn write_block_header(cursor: &mut WriteCursor, kind: u16) -> usize {
    let offset = cursor.len();
    cursor.write_u16_le(kind);
    cursor.write_u16_le(0);
    offset
}

fn finish_block_header(cursor: &mut WriteCursor, offset: usize) -> RdpResult<()> {
    let length = cursor.len() - offset;
    cursor.patch_u16_le(offset + 2, u16::try_from(length).map_err(|_| RdpProtocolError::encoding("GCC data block too long".to_string()))?)
}

fn write_client_core(cursor: &mut WriteCursor, settings: &ClientCoreSettings) -> RdpResult<()> {
    let offset = write_block_header(cursor, CS_CORE);
    cursor.write_u32_le(RDP_VERSION_5_PLUS);
    cursor.write_u16_le(settings.desktop_width);
    cursor.write_u16_le(settings.desktop_height);
    cursor.write_u16_le(RNS_UD_COLOR_8BPP);
    cursor.write_u16_le(RNS_UD_SAS_DEL);
    cursor.write_u32_le(settings.keyboard_layout);
    cursor.write_u32_le(CLIENT_BUILD);
    cursor.write_utf16_le_padded(&settings.client_name, 32)?;
    cursor.write_u32_le(KEYBOARD_TYPE_IBM_ENHANCED);
    cursor.write_u32_le(0);
    cursor.write_u32_le(KEYBOARD_FUNCTION_KEYS);
    cursor.write_zeros(64);
    cursor.write_u16_le(RNS_UD_COLOR_8BPP);
    cursor.write_u16_le(CLIENT_PRODUCT_ID);
    cursor.write_u32_le(0);
    cursor.write_u16_le(settings.color_depth);
    cursor.write_u16_le(RNS_UD_32BPP_SUPPORT | RNS_UD_24BPP_SUPPORT | RNS_UD_16BPP_SUPPORT | RNS_UD_15BPP_SUPPORT);
    cursor.write_u16_le(
        RNS_UD_CS_SUPPORT_ERRINFO_PDU
            | RNS_UD_CS_WANT_32BPP_SESSION
            | RNS_UD_CS_SUPPORT_STATUSINFO_PDU
            | RNS_UD_CS_STRONG_ASYMMETRIC_KEYS
            | RNS_UD_CS_VALID_CONNECTION_TYPE
            | RNS_UD_CS_SUPPORT_MONITOR_LAYOUT_PDU
            | RNS_UD_CS_SUPPORT_NETCHAR_AUTODETECT
            | RNS_UD_CS_SUPPORT_DYNVC_GFX_PROTOCOL
            | RNS_UD_CS_SUPPORT_HEARTBEAT_PDU,
    );
    cursor.write_zeros(64);
    cursor.write_u8(CONNECTION_TYPE_LAN);
    cursor.write_u8(0);
    cursor.write_u32_le(settings.selected_protocol);
    let primary = settings.monitors.iter().find(|monitor| monitor.primary).ok_or_else(|| RdpProtocolError::protocol("GCC monitor list has no primary monitor".to_string()))?;
    cursor.write_u32_le(primary.physical_width);
    cursor.write_u32_le(primary.physical_height);
    cursor.write_u16_le(u16::try_from(primary.orientation).map_err(|_| RdpProtocolError::encoding(format!("desktopOrientation cannot be encoded: {}", primary.orientation)))?);
    cursor.write_u32_le(primary.desktop_scale_factor);
    cursor.write_u32_le(primary.device_scale_factor);
    finish_block_header(cursor, offset)
}

fn write_client_message_channel(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = write_block_header(cursor, CS_MCS_MSGCHANNEL);
    cursor.write_u32_le(0);
    finish_block_header(cursor, offset)
}

fn write_client_multitransport(cursor: &mut WriteCursor) -> RdpResult<()> {
    let offset = write_block_header(cursor, CS_MULTITRANSPORT);
    cursor.write_u32_le(TRANSPORT_TYPE_UDP_FECR);
    finish_block_header(cursor, offset)
}

fn write_client_security(cursor: &mut WriteCursor, settings: &ClientCoreSettings) -> RdpResult<()> {
    let offset = write_block_header(cursor, CS_SECURITY);
    cursor.write_u32_le(settings.encryption_methods);
    cursor.write_u32_le(ENCRYPTION_METHOD_NONE);
    finish_block_header(cursor, offset)
}

fn write_client_cluster(cursor: &mut WriteCursor, settings: &ClientCoreSettings) -> RdpResult<()> {
    let offset = write_block_header(cursor, CS_CLUSTER);
    let mut flags = REDIRECTION_SUPPORTED | (REDIRECTION_VERSION4 << 2);
    if settings.console_session {
        flags |= REDIRECTED_SESSIONID_FIELD_VALID;
    }
    cursor.write_u32_le(flags);
    cursor.write_u32_le(0);
    finish_block_header(cursor, offset)
}

fn write_client_network(cursor: &mut WriteCursor, channels: &[ChannelDefinition]) -> RdpResult<()> {
    let offset = write_block_header(cursor, CS_NET);
    channel::write_client_network(cursor, channels)?;
    finish_block_header(cursor, offset)
}

fn write_client_monitor(cursor: &mut WriteCursor, settings: &ClientCoreSettings) -> RdpResult<()> {
    let primary = settings.monitors.iter().find(|monitor| monitor.primary).ok_or_else(|| RdpProtocolError::protocol("GCC monitor list has no primary monitor".to_string()))?;
    let offset = write_block_header(cursor, CS_MONITOR);
    cursor.write_u32_le(0);
    cursor.write_u32_le(u32::try_from(settings.monitors.len()).map_err(|_| RdpProtocolError::encoding(format!("too many monitors: {}", settings.monitors.len())))?);
    for monitor in &settings.monitors {
        let width = i32::try_from(monitor.width).map_err(|_| RdpProtocolError::encoding(format!("monitor width cannot be encoded: {}", monitor.width)))?;
        let height = i32::try_from(monitor.height).map_err(|_| RdpProtocolError::encoding(format!("monitor height cannot be encoded: {}", monitor.height)))?;
        let left = monitor.left.checked_sub(primary.left).ok_or_else(|| RdpProtocolError::encoding(format!("monitor left overflow: {} - {}", monitor.left, primary.left)))?;
        let top = monitor.top.checked_sub(primary.top).ok_or_else(|| RdpProtocolError::encoding(format!("monitor top overflow: {} - {}", monitor.top, primary.top)))?;
        let right = left.checked_add(width).and_then(|value| value.checked_sub(1)).ok_or_else(|| RdpProtocolError::encoding(format!("monitor right overflow: {}+{}", left, width)))?;
        let bottom = top.checked_add(height).and_then(|value| value.checked_sub(1)).ok_or_else(|| RdpProtocolError::encoding(format!("monitor bottom overflow: {}+{}", top, height)))?;
        cursor.write_u32_le(left as u32);
        cursor.write_u32_le(top as u32);
        cursor.write_u32_le(right as u32);
        cursor.write_u32_le(bottom as u32);
        cursor.write_u32_le(if monitor.primary { MONITOR_PRIMARY } else { 0 });
    }
    finish_block_header(cursor, offset)
}

fn write_client_monitor_ex(cursor: &mut WriteCursor, settings: &ClientCoreSettings) -> RdpResult<()> {
    if settings.monitors.is_empty() {
        return Err(RdpProtocolError::protocol("GCC monitor list is empty".to_string()));
    }
    let offset = write_block_header(cursor, CS_MONITOR_EX);
    cursor.write_u32_le(0);
    cursor.write_u32_le(MONITOR_ATTRIBUTE_SIZE);
    cursor.write_u32_le(u32::try_from(settings.monitors.len()).map_err(|_| RdpProtocolError::encoding(format!("too many monitors: {}", settings.monitors.len())))?);
    for monitor in &settings.monitors {
        cursor.write_u32_le(monitor.physical_width);
        cursor.write_u32_le(monitor.physical_height);
        cursor.write_u32_le(monitor.orientation);
        cursor.write_u32_le(monitor.desktop_scale_factor);
        cursor.write_u32_le(monitor.device_scale_factor);
    }
    finish_block_header(cursor, offset)
}

fn parse_server_data_blocks(blocks: &[u8]) -> RdpResult<ServerData> {
    let mut cursor = ReadCursor::new(blocks);
    let mut data = ServerData { encryption_method: ENCRYPTION_METHOD_NONE, encryption_level: ENCRYPTION_LEVEL_NONE, ..ServerData::default() };
    while cursor.remaining() >= 4 {
        let kind = cursor.read_u16_le()?;
        let length = cursor.read_u16_le()? as usize;
        if length < 4 {
            return Err(RdpProtocolError::protocol(format!("unexpected GCC server data block length: {}", length)));
        }
        let mut block = cursor.sub_cursor(length - 4)?;
        match kind {
            SC_CORE => {
                data.version = block.read_u32_le()?;
            }
            SC_SECURITY => {
                data.encryption_method = block.read_u32_le()?;
                data.encryption_level = block.read_u32_le()?;
                if data.encryption_level != ENCRYPTION_LEVEL_NONE {
                    let random_length = block.read_u32_le()? as usize;
                    let certificate_length = block.read_u32_le()? as usize;
                    data.server_random = block.read_vec(random_length)?;
                    data.server_certificate = block.read_vec(certificate_length)?;
                }
            }
            SC_MCS_MSGCHANNEL => {
                data.message_channel_id = block.read_u16_le()?;
            }
            SC_NET => {
                data.mcs_channel_id = block.read_u16_le()?;
                let count = block.read_u16_le()? as usize;
                for _ in 0..count {
                    data.channel_ids.push(block.read_u16_le()?);
                }
            }
            _ => {}
        }
    }
    if data.mcs_channel_id == 0 {
        return Err(RdpProtocolError::protocol("GCC response is missing the server network data block".to_string()));
    }
    Ok(data)
}
