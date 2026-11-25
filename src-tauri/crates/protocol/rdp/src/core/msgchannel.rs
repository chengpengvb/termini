use std::time::Instant;
use crate::core::security;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

const TYPE_ID_AUTODETECT_REQUEST: u8 = 0x00;
const TYPE_ID_AUTODETECT_RESPONSE: u8 = 0x01;

const RDP_RTT_REQUEST_TYPE_CONTINUOUS: u16 = 0x0001;
const RDP_RTT_REQUEST_TYPE_CONNECTTIME: u16 = 0x1001;
const RDP_RTT_RESPONSE_TYPE: u16 = 0x0000;

const RDP_BW_START_REQUEST_TYPE_CONTINUOUS: u16 = 0x0014;
const RDP_BW_START_REQUEST_TYPE_TUNNEL: u16 = 0x0114;
const RDP_BW_START_REQUEST_TYPE_CONNECTTIME: u16 = 0x1014;
const RDP_BW_PAYLOAD_REQUEST_TYPE: u16 = 0x0002;
const RDP_BW_STOP_REQUEST_TYPE_CONNECTTIME: u16 = 0x002B;
const RDP_BW_STOP_REQUEST_TYPE_CONTINUOUS: u16 = 0x0429;
const RDP_BW_STOP_REQUEST_TYPE_TUNNEL: u16 = 0x0629;
const RDP_BW_RESULTS_RESPONSE_TYPE_CONNECTTIME: u16 = 0x0003;
const RDP_BW_RESULTS_RESPONSE_TYPE_CONTINUOUS: u16 = 0x000B;

const RDP_NETCHAR_RESULTS_BASE_RTT: u16 = 0x0840;
const RDP_NETCHAR_RESULTS_BW_AVG_RTT: u16 = 0x0880;
const RDP_NETCHAR_RESULTS_BASE_BW_AVG_RTT: u16 = 0x08C0;
const RDP_NETCHAR_SYNC_RESPONSE_TYPE: u16 = 0x0018;

const HEADER_LENGTH_BASE: u8 = 0x06;
const HEADER_LENGTH_PAYLOAD: u8 = 0x08;
const HEADER_LENGTH_RESULTS: u8 = 0x0E;
const HEADER_LENGTH_NETCHAR_FULL: u8 = 0x12;

const HEARTBEAT_LENGTH: usize = 4;
const MULTITRANSPORT_COOKIE_LENGTH: usize = 16;
const MULTITRANSPORT_ABORT: u32 = 0x8000_4004;

#[derive(Default)]
pub struct MessageChannelState {
    measure_start: Option<Instant>,
    byte_count: u32,
}

pub fn process(state: &mut MessageChannelState, payload: &[u8]) -> RdpResult<Option<Vec<u8>>> {
    let mut cursor = ReadCursor::new(payload);
    let flags = security::read_header(&mut cursor)?;
    if flags & security::SEC_ENCRYPT != 0 {
        return Err(RdpProtocolError::protocol(format!("message channel security header still carries encryption flags: 0x{:04x}", flags)));
    }
    if flags & security::SEC_AUTODETECT_REQ != 0 {
        return handle_autodetect_request(state, &mut cursor);
    }
    if flags & security::SEC_AUTODETECT_RSP != 0 {
        return handle_autodetect_response(&mut cursor);
    }
    if flags & security::SEC_HEARTBEAT != 0 {
        cursor.skip(HEARTBEAT_LENGTH)?;
        return Ok(None);
    }
    if flags & security::SEC_TRANSPORT_REQ != 0 {
        return handle_multitransport_request(&mut cursor);
    }
    Err(RdpProtocolError::protocol(format!("message channel received unknown security flags: 0x{:04x}", flags)))
}

fn handle_autodetect_request(state: &mut MessageChannelState, cursor: &mut ReadCursor<'_>) -> RdpResult<Option<Vec<u8>>> {
    let header_length = cursor.read_u8()?;
    let header_type = cursor.read_u8()?;
    let sequence = cursor.read_u16_le()?;
    let request_type = cursor.read_u16_le()?;
    if header_type != TYPE_ID_AUTODETECT_REQUEST {
        return Err(RdpProtocolError::protocol(format!("unexpected auto-detect request header type: 0x{:02x}", header_type)));
    }
    match request_type {
        RDP_RTT_REQUEST_TYPE_CONTINUOUS | RDP_RTT_REQUEST_TYPE_CONNECTTIME => {
            expect_header_length(header_length, HEADER_LENGTH_BASE)?;
            Ok(Some(build_rtt_response(sequence)))
        }
        RDP_BW_START_REQUEST_TYPE_CONTINUOUS | RDP_BW_START_REQUEST_TYPE_TUNNEL | RDP_BW_START_REQUEST_TYPE_CONNECTTIME => {
            expect_header_length(header_length, HEADER_LENGTH_BASE)?;
            state.measure_start = Some(Instant::now());
            state.byte_count = 0;
            Ok(None)
        }
        RDP_BW_PAYLOAD_REQUEST_TYPE => {
            expect_header_length(header_length, HEADER_LENGTH_PAYLOAD)?;
            state.byte_count += read_measure_payload(cursor)?;
            Ok(None)
        }
        RDP_BW_STOP_REQUEST_TYPE_CONNECTTIME => {
            expect_header_length(header_length, HEADER_LENGTH_PAYLOAD)?;
            state.byte_count += read_measure_payload(cursor)?;
            build_bandwidth_results(state, sequence, RDP_BW_RESULTS_RESPONSE_TYPE_CONNECTTIME).map(Some)
        }
        RDP_BW_STOP_REQUEST_TYPE_CONTINUOUS | RDP_BW_STOP_REQUEST_TYPE_TUNNEL => {
            expect_header_length(header_length, HEADER_LENGTH_BASE)?;
            build_bandwidth_results(state, sequence, RDP_BW_RESULTS_RESPONSE_TYPE_CONTINUOUS).map(Some)
        }
        RDP_NETCHAR_RESULTS_BASE_RTT | RDP_NETCHAR_RESULTS_BW_AVG_RTT => {
            expect_header_length(header_length, HEADER_LENGTH_RESULTS)?;
            cursor.skip(8)?;
            Ok(None)
        }
        RDP_NETCHAR_RESULTS_BASE_BW_AVG_RTT => {
            expect_header_length(header_length, HEADER_LENGTH_NETCHAR_FULL)?;
            cursor.skip(12)?;
            Ok(None)
        }
        other => Err(RdpProtocolError::protocol(format!("unknown auto-detect request type: 0x{:04x}", other))),
    }
}

fn handle_autodetect_response(cursor: &mut ReadCursor<'_>) -> RdpResult<Option<Vec<u8>>> {
    let header_length = cursor.read_u8()?;
    let header_type = cursor.read_u8()?;
    cursor.read_u16_le()?;
    let response_type = cursor.read_u16_le()?;
    if header_type != TYPE_ID_AUTODETECT_RESPONSE {
        return Err(RdpProtocolError::protocol(format!("unexpected auto-detect response header type: 0x{:02x}", header_type)));
    }
    match response_type {
        RDP_NETCHAR_SYNC_RESPONSE_TYPE => {
            expect_header_length(header_length, HEADER_LENGTH_RESULTS)?;
            cursor.skip(8)?;
            Ok(None)
        }
        other => Err(RdpProtocolError::protocol(format!("client received unexpected auto-detect response type: 0x{:04x}", other))),
    }
}

fn handle_multitransport_request(cursor: &mut ReadCursor<'_>) -> RdpResult<Option<Vec<u8>>> {
    let request_id = cursor.read_u32_le()?;
    cursor.read_u16_le()?;
    cursor.read_u16_le()?;
    cursor.skip(MULTITRANSPORT_COOKIE_LENGTH)?;
    let mut response = WriteCursor::with_capacity(12);
    security::write_header(&mut response, security::SEC_TRANSPORT_RSP);
    response.write_u32_le(request_id);
    response.write_u32_le(MULTITRANSPORT_ABORT);
    Ok(Some(response.into_vec()))
}

fn read_measure_payload(cursor: &mut ReadCursor<'_>) -> RdpResult<u32> {
    let payload_length = cursor.read_u16_le()?;
    cursor.skip(usize::from(payload_length))?;
    Ok(u32::from(payload_length))
}

fn build_rtt_response(sequence: u16) -> Vec<u8> {
    let mut cursor = WriteCursor::with_capacity(10);
    security::write_header(&mut cursor, security::SEC_AUTODETECT_RSP);
    cursor.write_u8(HEADER_LENGTH_BASE);
    cursor.write_u8(TYPE_ID_AUTODETECT_RESPONSE);
    cursor.write_u16_le(sequence);
    cursor.write_u16_le(RDP_RTT_RESPONSE_TYPE);
    cursor.into_vec()
}

fn build_bandwidth_results(state: &mut MessageChannelState, sequence: u16, response_type: u16) -> RdpResult<Vec<u8>> {
    let start = state.measure_start.take().ok_or_else(|| RdpProtocolError::protocol("received bandwidth measure stop without a start".to_string()))?;
    let elapsed = start.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
    let mut cursor = WriteCursor::with_capacity(18);
    security::write_header(&mut cursor, security::SEC_AUTODETECT_RSP);
    cursor.write_u8(HEADER_LENGTH_RESULTS);
    cursor.write_u8(TYPE_ID_AUTODETECT_RESPONSE);
    cursor.write_u16_le(sequence);
    cursor.write_u16_le(response_type);
    cursor.write_u32_le(elapsed);
    cursor.write_u32_le(state.byte_count);
    Ok(cursor.into_vec())
}

fn expect_header_length(actual: u8, expected: u8) -> RdpResult<()> {
    if actual != expected {
        return Err(RdpProtocolError::protocol(format!("unexpected auto-detect header length: expected 0x{:02x} got 0x{:02x}", expected, actual)));
    }
    Ok(())
}
