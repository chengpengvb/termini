use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

const RDPDR_CTYP_CORE: u16 = 0x4472;

const PAKID_CORE_SERVER_ANNOUNCE: u16 = 0x496E;
const PAKID_CORE_CLIENTID_CONFIRM: u16 = 0x4343;
const PAKID_CORE_CLIENT_NAME: u16 = 0x434E;
const PAKID_CORE_DEVICELIST_ANNOUNCE: u16 = 0x4441;
const PAKID_CORE_DEVICE_REPLY: u16 = 0x6472;
const PAKID_CORE_DEVICE_IOREQUEST: u16 = 0x4952;
const PAKID_CORE_DEVICE_IOCOMPLETION: u16 = 0x4943;
const PAKID_CORE_SERVER_CAPABILITY: u16 = 0x5350;
const PAKID_CORE_CLIENT_CAPABILITY: u16 = 0x4350;
const PAKID_CORE_USER_LOGGEDON: u16 = 0x554C;

const CAP_GENERAL_TYPE: u16 = 0x0001;
const GENERAL_CAPABILITY_VERSION_02: u32 = 0x0000_0002;
const GENERAL_CAPABILITY_LENGTH: u16 = 44;
const CAPABILITY_HEADER_LENGTH: u16 = 8;

const CLIENT_VERSION_MAJOR: u16 = 0x0001;
const CLIENT_VERSION_MINOR: u16 = 0x000D;

const RDPDR_DEVICE_REMOVE_PDUS: u32 = 0x0000_0001;
const RDPDR_CLIENT_DISPLAY_NAME_PDU: u32 = 0x0000_0002;
const RDPDR_USER_LOGGEDON_PDU: u32 = 0x0000_0004;
const CLIENT_EXTENDED_PDU: u32 = RDPDR_DEVICE_REMOVE_PDUS | RDPDR_CLIENT_DISPLAY_NAME_PDU | RDPDR_USER_LOGGEDON_PDU;

const RDPDR_IRP_MJ_CREATE: u32 = 0x0000_0001;
const RDPDR_IRP_MJ_CLEANUP: u32 = 0x0000_0002;
const RDPDR_IRP_MJ_CLOSE: u32 = 0x0000_0004;
const RDPDR_IRP_MJ_READ: u32 = 0x0000_0008;
const RDPDR_IRP_MJ_WRITE: u32 = 0x0000_0010;
const RDPDR_IRP_MJ_FLUSH_BUFFERS: u32 = 0x0000_0020;
const RDPDR_IRP_MJ_SHUTDOWN: u32 = 0x0000_0040;
const RDPDR_IRP_MJ_DEVICE_CONTROL: u32 = 0x0000_0080;
const RDPDR_IRP_MJ_QUERY_VOLUME_INFORMATION: u32 = 0x0000_0100;
const RDPDR_IRP_MJ_SET_VOLUME_INFORMATION: u32 = 0x0000_0200;
const RDPDR_IRP_MJ_QUERY_INFORMATION: u32 = 0x0000_0400;
const RDPDR_IRP_MJ_SET_INFORMATION: u32 = 0x0000_0800;
const RDPDR_IRP_MJ_DIRECTORY_CONTROL: u32 = 0x0000_1000;
const RDPDR_IRP_MJ_LOCK_CONTROL: u32 = 0x0000_2000;
const RDPDR_IRP_MJ_QUERY_SECURITY: u32 = 0x0000_4000;
const RDPDR_IRP_MJ_SET_SECURITY: u32 = 0x0000_8000;
const CLIENT_IO_CODE1: u32 = RDPDR_IRP_MJ_CREATE
    | RDPDR_IRP_MJ_CLEANUP
    | RDPDR_IRP_MJ_CLOSE
    | RDPDR_IRP_MJ_READ
    | RDPDR_IRP_MJ_WRITE
    | RDPDR_IRP_MJ_FLUSH_BUFFERS
    | RDPDR_IRP_MJ_SHUTDOWN
    | RDPDR_IRP_MJ_DEVICE_CONTROL
    | RDPDR_IRP_MJ_QUERY_VOLUME_INFORMATION
    | RDPDR_IRP_MJ_SET_VOLUME_INFORMATION
    | RDPDR_IRP_MJ_QUERY_INFORMATION
    | RDPDR_IRP_MJ_SET_INFORMATION
    | RDPDR_IRP_MJ_DIRECTORY_CONTROL
    | RDPDR_IRP_MJ_LOCK_CONTROL
    | RDPDR_IRP_MJ_QUERY_SECURITY
    | RDPDR_IRP_MJ_SET_SECURITY;

const ENABLE_ASYNCIO: u32 = 0x0000_0001;
const UNICODE_FLAG: u32 = 0x0000_0001;

const IO_COMPLETION_HEADER_LENGTH: usize = 16;
const STATUS_UNSUCCESSFUL: u32 = 0xC000_0001;

const RDPDR_DTYP_FILESYSTEM: u32 = 0x0000_0008;
const PREFERRED_DOS_NAME_LENGTH: usize = 8;

pub const DRIVE_DEVICE_ID: u32 = 1;

pub struct DeviceIoRequest {
    pub file_id: u32,
    pub device_id: u32,
    pub completion_id: u32,
    pub major: u32,
    pub minor: u32,
    pub payload: Vec<u8>,
}

#[derive(Default)]
pub struct RdpdrOutcome {
    pub responses: Vec<Vec<u8>>,
    pub request: Option<DeviceIoRequest>,
}

#[derive(Default)]
pub struct RdpdrState {
    client_id: u32,
    io_code1: u32,
    version_major: u16,
    version_minor: u16,
    have_client_id: bool,
    have_server_caps: bool,
    user_logged_on: bool,
    announced: bool,
}

pub fn process(state: &mut RdpdrState, machine_name: &str, drive: Option<&str>, message: &[u8]) -> RdpResult<RdpdrOutcome> {
    let mut cursor = ReadCursor::new(message);
    let component = cursor.read_u16_le()?;
    let packet_id = cursor.read_u16_le()?;
    if component != RDPDR_CTYP_CORE {
        return Err(RdpProtocolError::unsupported(format!("rdpdr component 0x{:04x} not implemented", component)));
    }
    match packet_id {
        PAKID_CORE_SERVER_ANNOUNCE => {
            read_server_announce(state, &mut cursor)?;
            Ok(RdpdrOutcome { responses: vec![build_client_id_confirm(state), build_client_name(machine_name)?], request: None })
        }
        PAKID_CORE_SERVER_CAPABILITY => {
            read_server_capability_sets(state, &mut cursor)?;
            state.have_server_caps = true;
            let mut responses = vec![build_client_capability(state)];
            responses.extend(build_device_list(state, drive)?);
            Ok(RdpdrOutcome { responses, request: None })
        }
        PAKID_CORE_CLIENTID_CONFIRM => {
            state.have_client_id = true;
            Ok(RdpdrOutcome { responses: build_device_list(state, drive)?, request: None })
        }
        PAKID_CORE_USER_LOGGEDON => {
            state.user_logged_on = true;
            Ok(RdpdrOutcome { responses: build_device_list(state, drive)?, request: None })
        }
        PAKID_CORE_DEVICE_REPLY => {
            let device_id = cursor.read_u32_le()?;
            let status = cursor.read_u32_le()?;
            if status != 0 {
                return Err(RdpProtocolError::rejected(format!("server rejected redirected device {}: 0x{:08x}", device_id, status)));
            }
            Ok(RdpdrOutcome::default())
        }
        PAKID_CORE_DEVICE_IOREQUEST => read_io_request(&mut cursor),
        other => Err(RdpProtocolError::unsupported(format!("rdpdr message 0x{:04x} not implemented", other))),
    }
}

pub fn build_io_completion(device_id: u32, completion_id: u32, status: u32, body: &[u8]) -> Vec<u8> {
    let mut cursor = WriteCursor::with_capacity(IO_COMPLETION_HEADER_LENGTH + body.len());
    write_header(&mut cursor, PAKID_CORE_DEVICE_IOCOMPLETION);
    cursor.write_u32_le(device_id);
    cursor.write_u32_le(completion_id);
    cursor.write_u32_le(status);
    cursor.write_slice(body);
    cursor.into_vec()
}

fn read_io_request(cursor: &mut ReadCursor<'_>) -> RdpResult<RdpdrOutcome> {
    let device_id = cursor.read_u32_le()?;
    let file_id = cursor.read_u32_le()?;
    let completion_id = cursor.read_u32_le()?;
    let major = cursor.read_u32_le()?;
    let minor = cursor.read_u32_le()?;
    if device_id != DRIVE_DEVICE_ID {
        return Ok(RdpdrOutcome { responses: vec![build_io_completion(device_id, completion_id, STATUS_UNSUCCESSFUL, &[])], request: None });
    }
    let payload = cursor.rest().to_vec();
    Ok(RdpdrOutcome { responses: Vec::new(), request: Some(DeviceIoRequest { device_id, file_id, completion_id, major, minor, payload }) })
}

fn build_device_list(state: &mut RdpdrState, drive: Option<&str>) -> RdpResult<Vec<Vec<u8>>> {
    let Some(drive) = drive else {
        return Ok(Vec::new());
    };
    if state.announced || !state.have_client_id || !state.have_server_caps || !state.user_logged_on {
        return Ok(Vec::new());
    }
    state.announced = true;
    let label = drive_label(drive);
    let mut name = label.clone().into_bytes();
    name.push(0);

    let mut cursor = WriteCursor::with_capacity(name.len() + 32);
    write_header(&mut cursor, PAKID_CORE_DEVICELIST_ANNOUNCE);
    cursor.write_u32_le(1);
    cursor.write_u32_le(RDPDR_DTYP_FILESYSTEM);
    cursor.write_u32_le(DRIVE_DEVICE_ID);
    let dos_name = label.as_bytes();
    let padding = PREFERRED_DOS_NAME_LENGTH.checked_sub(dos_name.len()).ok_or_else(|| RdpProtocolError::encoding(format!("rdpdr device name too long: {}", label)))?;
    cursor.write_slice(dos_name);
    cursor.write_zeros(padding);
    cursor.write_u32_le(u32::try_from(name.len()).map_err(|_| RdpProtocolError::encoding(format!("rdpdr device data too long: {}", name.len())))?);
    cursor.write_slice(&name);
    Ok(vec![cursor.into_vec()])
}

fn drive_label(drive: &str) -> String {
    let name = drive.rsplit(['/', '\\']).find(|part| !part.is_empty()).unwrap_or(drive);
    name.chars().filter(|value| value.is_ascii_alphanumeric()).take(PREFERRED_DOS_NAME_LENGTH).collect::<String>().to_uppercase()
}

fn read_server_announce(state: &mut RdpdrState, cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
    let version_major = cursor.read_u16_le()?;
    let version_minor = cursor.read_u16_le()?;
    state.version_major = version_major.min(CLIENT_VERSION_MAJOR);
    state.version_minor = version_minor.min(CLIENT_VERSION_MINOR);
    state.client_id = cursor.read_u32_le()?;
    Ok(())
}

fn read_server_capability_sets(state: &mut RdpdrState, cursor: &mut ReadCursor<'_>) -> RdpResult<()> {
    let count = cursor.read_u16_le()?;
    cursor.skip(2)?;
    for _ in 0..count {
        let kind = cursor.read_u16_le()?;
        let length = cursor.read_u16_le()?;
        if length < CAPABILITY_HEADER_LENGTH {
            return Err(RdpProtocolError::protocol(format!("rdpdr capability set too short: {}", length)));
        }
        cursor.read_u32_le()?;
        let mut body = cursor.sub_cursor(usize::from(length - CAPABILITY_HEADER_LENGTH))?;
        if kind == CAP_GENERAL_TYPE {
            body.skip(12)?;
            state.io_code1 = body.read_u32_le()?;
        }
    }
    Ok(())
}

fn build_client_id_confirm(state: &RdpdrState) -> Vec<u8> {
    let mut cursor = WriteCursor::with_capacity(12);
    write_header(&mut cursor, PAKID_CORE_CLIENTID_CONFIRM);
    cursor.write_u16_le(state.version_major);
    cursor.write_u16_le(state.version_minor);
    cursor.write_u32_le(state.client_id);
    cursor.into_vec()
}

fn build_client_name(machine_name: &str) -> RdpResult<Vec<u8>> {
    let mut cursor = WriteCursor::with_capacity(machine_name.len() * 2 + 20);
    write_header(&mut cursor, PAKID_CORE_CLIENT_NAME);
    cursor.write_u32_le(UNICODE_FLAG);
    cursor.write_u32_le(0);
    let length = (machine_name.encode_utf16().count() + 1) * 2;
    cursor.write_u32_le(u32::try_from(length).map_err(|_| RdpProtocolError::encoding(format!("rdpdr computer name too long: {}", length)))?);
    cursor.write_utf16_le(machine_name);
    cursor.write_u16_le(0);
    Ok(cursor.into_vec())
}

fn build_client_capability(state: &RdpdrState) -> Vec<u8> {
    let mut cursor = WriteCursor::with_capacity(usize::from(GENERAL_CAPABILITY_LENGTH) + 8);
    write_header(&mut cursor, PAKID_CORE_CLIENT_CAPABILITY);
    cursor.write_u16_le(1);
    cursor.write_u16_le(0);
    cursor.write_u16_le(CAP_GENERAL_TYPE);
    cursor.write_u16_le(GENERAL_CAPABILITY_LENGTH);
    cursor.write_u32_le(GENERAL_CAPABILITY_VERSION_02);
    cursor.write_u32_le(0);
    cursor.write_u32_le(0);
    cursor.write_u16_le(state.version_major);
    cursor.write_u16_le(state.version_minor);
    cursor.write_u32_le(CLIENT_IO_CODE1 & state.io_code1);
    cursor.write_u32_le(0);
    cursor.write_u32_le(CLIENT_EXTENDED_PDU);
    cursor.write_u32_le(ENABLE_ASYNCIO);
    cursor.write_u32_le(0);
    cursor.write_u32_le(0);
    cursor.into_vec()
}

fn write_header(cursor: &mut WriteCursor, packet_id: u16) {
    cursor.write_u16_le(RDPDR_CTYP_CORE);
    cursor.write_u16_le(packet_id);
}
