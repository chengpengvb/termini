use crate::core::gcc::Monitor;
use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

pub const CHANNEL_NAME: &str = "Microsoft::Windows::RDS::DisplayControl";
pub const DEFAULT_MAX_NUM_MONITORS: u32 = 16;

const PDU_TYPE_MONITOR_LAYOUT: u32 = 0x0000_0002;
const PDU_TYPE_CAPS: u32 = 0x0000_0005;
const HEADER_LENGTH: u32 = 8;
const MONITOR_LAYOUT_SIZE: u32 = 40;
const MONITOR_PRIMARY: u32 = 0x0000_0001;
const MONITOR_MIN_EDGE: u32 = 200;
const MONITOR_MAX_EDGE: u32 = 8192;

pub fn parse_caps(message: &[u8]) -> RdpResult<u32> {
    let mut cursor = ReadCursor::new(message);
    let pdu_type = cursor.read_u32_le()?;
    let length = cursor.read_u32_le()?;
    if length < HEADER_LENGTH {
        return Err(RdpProtocolError::protocol(format!("disp header too short: {}", length)));
    }
    if pdu_type != PDU_TYPE_CAPS {
        return Err(RdpProtocolError::unsupported(format!("disp pdu type {} not implemented", pdu_type)));
    }
    let max_num_monitors = cursor.read_u32_le()?;
    cursor.skip(8)?;
    Ok(max_num_monitors)
}

pub fn build_monitor_layout(monitors: &[Monitor]) -> RdpResult<Vec<u8>> {
    if monitors.is_empty() {
        return Err(RdpProtocolError::protocol("disp monitor list is empty".to_string()));
    }
    let count = u32::try_from(monitors.len()).map_err(|_| RdpProtocolError::encoding(format!("disp too many monitors: {}", monitors.len())))?;
    let length = HEADER_LENGTH + 8 + MONITOR_LAYOUT_SIZE * count;
    let mut cursor = WriteCursor::with_capacity(length as usize);
    cursor.write_u32_le(PDU_TYPE_MONITOR_LAYOUT);
    cursor.write_u32_le(length);
    cursor.write_u32_le(MONITOR_LAYOUT_SIZE);
    cursor.write_u32_le(count);
    for monitor in monitors {
        cursor.write_u32_le(if monitor.primary { MONITOR_PRIMARY } else { 0 });
        cursor.write_u32_le(monitor.left as u32);
        cursor.write_u32_le(monitor.top as u32);
        cursor.write_u32_le(layout_width(monitor.width)?);
        cursor.write_u32_le(layout_height(monitor.height)?);
        cursor.write_u32_le(monitor.physical_width);
        cursor.write_u32_le(monitor.physical_height);
        cursor.write_u32_le(monitor.orientation);
        cursor.write_u32_le(monitor.desktop_scale_factor);
        cursor.write_u32_le(monitor.device_scale_factor);
    }
    Ok(cursor.into_vec())
}

fn layout_width(pixels: u32) -> RdpResult<u32> {
    layout_edge(pixels & !1, "width")
}

fn layout_height(pixels: u32) -> RdpResult<u32> {
    layout_edge(pixels, "height")
}

fn layout_edge(pixels: u32, axis: &str) -> RdpResult<u32> {
    if pixels < MONITOR_MIN_EDGE || pixels > MONITOR_MAX_EDGE {
        return Err(RdpProtocolError::protocol(format!("disp monitor {} out of range: {}", axis, pixels)));
    }
    Ok(pixels)
}
