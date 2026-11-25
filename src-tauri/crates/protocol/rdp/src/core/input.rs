use crate::core::pdu;
use crate::core::fastpath;
use crate::core::stream::WriteCursor;
use crate::core::error::{RdpProtocolError, RdpResult};
use crate::core::capabilities::{INPUT_FLAG_MOUSEX, INPUT_FLAG_MOUSE_HWHEEL, INPUT_FLAG_MOUSE_RELATIVE, INPUT_FLAG_UNICODE};

pub const KBDFLAGS_RELEASE: u8 = 0x01;
pub const KBDFLAGS_EXTENDED: u8 = 0x02;

pub const SYNC_SCROLL_LOCK: u8 = 0x01;
pub const SYNC_NUM_LOCK: u8 = 0x02;
pub const SYNC_CAPS_LOCK: u8 = 0x04;
pub const SYNC_KANA_LOCK: u8 = 0x08;

pub const PTR_FLAGS_HWHEEL: u16 = 0x0400;
pub const PTR_FLAGS_WHEEL: u16 = 0x0200;
pub const PTR_FLAGS_WHEEL_NEGATIVE: u16 = 0x0100;
pub const PTR_FLAGS_MOVE: u16 = 0x0800;
pub const PTR_FLAGS_DOWN: u16 = 0x8000;
pub const PTR_FLAGS_BUTTON1: u16 = 0x1000;
pub const PTR_FLAGS_BUTTON2: u16 = 0x2000;
pub const PTR_FLAGS_BUTTON3: u16 = 0x4000;
pub const PTR_XFLAGS_DOWN: u16 = 0x8000;
pub const PTR_XFLAGS_BUTTON1: u16 = 0x0001;
pub const PTR_XFLAGS_BUTTON2: u16 = 0x0002;

const WHEEL_ROTATION_MASK: i16 = 0x01FF;
const SLOWPATH_EVENT_SYNC: u16 = 0x0000;
const SLOWPATH_EVENT_SCANCODE: u16 = 0x0004;
const SLOWPATH_EVENT_UNICODE: u16 = 0x0005;
const SLOWPATH_EVENT_MOUSE: u16 = 0x8001;
const SLOWPATH_EVENT_MOUSEX: u16 = 0x8002;
const SLOWPATH_EVENT_MOUSE_RELATIVE: u16 = 0x8004;
const SLOWPATH_KBDFLAGS_EXTENDED: u16 = 0x0100;
const SLOWPATH_KBDFLAGS_RELEASE: u16 = 0x8000;

pub fn scancode_event(code: u16, pressed: bool, extended: bool) -> RdpResult<Vec<u8>> {
    let key_code = u8::try_from(code).map_err(|_| RdpProtocolError::encoding(format!("scancode exceeds single-byte range: {}", code)))?;
    let mut flags = 0u8;
    if !pressed {
        flags |= KBDFLAGS_RELEASE;
    }
    if extended {
        flags |= KBDFLAGS_EXTENDED;
    }
    let mut cursor = WriteCursor::with_capacity(2);
    cursor.write_u8(event_header(fastpath::INPUT_EVENT_SCANCODE, flags));
    cursor.write_u8(key_code);
    Ok(cursor.into_vec())
}

pub fn unicode_event(code: u16, pressed: bool, input_flags: u16) -> RdpResult<Vec<u8>> {
    if input_flags & INPUT_FLAG_UNICODE == 0 {
        return Err(RdpProtocolError::unsupported("server did not negotiate unicode input".to_string()));
    }
    let flags = match pressed {
        true => 0u8,
        false => KBDFLAGS_RELEASE,
    };
    let mut cursor = WriteCursor::with_capacity(3);
    cursor.write_u8(event_header(fastpath::INPUT_EVENT_UNICODE, flags));
    cursor.write_u16_le(code);
    Ok(cursor.into_vec())
}

pub fn sync_event(scroll_lock: bool, num_lock: bool, caps_lock: bool, kana_lock: bool) -> Vec<u8> {
    let mut flags = 0u8;
    if scroll_lock {
        flags |= SYNC_SCROLL_LOCK;
    }
    if num_lock {
        flags |= SYNC_NUM_LOCK;
    }
    if caps_lock {
        flags |= SYNC_CAPS_LOCK;
    }
    if kana_lock {
        flags |= SYNC_KANA_LOCK;
    }
    vec![event_header(fastpath::INPUT_EVENT_SYNC, flags)]
}

pub fn move_event(x: u16, y: u16) -> Vec<u8> {
    mouse_event(PTR_FLAGS_MOVE, x, y)
}

pub fn move_relative_event(x: i16, y: i16, input_flags: u16) -> RdpResult<Vec<u8>> {
    if input_flags & INPUT_FLAG_MOUSE_RELATIVE == 0 {
        return Err(RdpProtocolError::unsupported("server did not negotiate relative mouse events".to_string()));
    }
    Ok(write_pointer_event(fastpath::INPUT_EVENT_MOUSE_RELATIVE, PTR_FLAGS_MOVE, x as u16, y as u16))
}

pub fn button_event(button: u8, pressed: bool, x: u16, y: u16, input_flags: u16) -> RdpResult<Vec<u8>> {
    let (extended, flags) = pointer_button(button, pressed, input_flags)?;
    if extended {
        Ok(mouse_extended_event(flags, x, y))
    } else {
        Ok(mouse_event(flags, x, y))
    }
}

pub fn wheel_event(delta: i16, horizontal: bool, x: u16, y: u16, input_flags: u16) -> RdpResult<Vec<u8>> {
    Ok(mouse_event(wheel_flags(delta, horizontal, input_flags)?, x, y))
}

pub fn slowpath_scancode_event(code: u16, pressed: bool, extended: bool) -> RdpResult<Vec<u8>> {
    let key_code = u8::try_from(code).map_err(|_| RdpProtocolError::encoding(format!("scancode exceeds single-byte range: {}", code)))?;
    let mut flags = 0u16;
    if !pressed {
        flags |= SLOWPATH_KBDFLAGS_RELEASE;
    }
    if extended {
        flags |= SLOWPATH_KBDFLAGS_EXTENDED;
    }
    Ok(write_slowpath_event(SLOWPATH_EVENT_SCANCODE, |cursor| {
        cursor.write_u16_le(flags);
        cursor.write_u16_le(u16::from(key_code));
        cursor.write_u16_le(0);
    }))
}

pub fn slowpath_unicode_event(code: u16, pressed: bool, input_flags: u16) -> RdpResult<Vec<u8>> {
    if input_flags & INPUT_FLAG_UNICODE == 0 {
        return Err(RdpProtocolError::unsupported("server did not negotiate unicode input".to_string()));
    }
    let flags = match pressed {
        true => 0u16,
        false => SLOWPATH_KBDFLAGS_RELEASE,
    };
    Ok(write_slowpath_event(SLOWPATH_EVENT_UNICODE, |cursor| {
        cursor.write_u16_le(flags);
        cursor.write_u16_le(code);
        cursor.write_u16_le(0);
    }))
}

pub fn slowpath_sync_event(scroll_lock: bool, num_lock: bool, caps_lock: bool, kana_lock: bool) -> Vec<u8> {
    let mut flags = 0u32;
    if scroll_lock {
        flags |= u32::from(SYNC_SCROLL_LOCK);
    }
    if num_lock {
        flags |= u32::from(SYNC_NUM_LOCK);
    }
    if caps_lock {
        flags |= u32::from(SYNC_CAPS_LOCK);
    }
    if kana_lock {
        flags |= u32::from(SYNC_KANA_LOCK);
    }
    write_slowpath_event(SLOWPATH_EVENT_SYNC, |cursor| {
        cursor.write_u16_le(0);
        cursor.write_u32_le(flags);
    })
}

pub fn slowpath_move_event(x: u16, y: u16) -> Vec<u8> {
    write_slowpath_pointer(SLOWPATH_EVENT_MOUSE, PTR_FLAGS_MOVE, x, y)
}

pub fn slowpath_move_relative_event(x: i16, y: i16, input_flags: u16) -> RdpResult<Vec<u8>> {
    if input_flags & INPUT_FLAG_MOUSE_RELATIVE == 0 {
        return Err(RdpProtocolError::unsupported("server did not negotiate relative mouse events".to_string()));
    }
    Ok(write_slowpath_pointer(SLOWPATH_EVENT_MOUSE_RELATIVE, PTR_FLAGS_MOVE, x as u16, y as u16))
}

pub fn slowpath_button_event(button: u8, pressed: bool, x: u16, y: u16, input_flags: u16) -> RdpResult<Vec<u8>> {
    let (extended, flags) = pointer_button(button, pressed, input_flags)?;
    let message_type = match extended {
        true => SLOWPATH_EVENT_MOUSEX,
        false => SLOWPATH_EVENT_MOUSE,
    };
    Ok(write_slowpath_pointer(message_type, flags, x, y))
}

pub fn slowpath_wheel_event(delta: i16, horizontal: bool, x: u16, y: u16, input_flags: u16) -> RdpResult<Vec<u8>> {
    Ok(write_slowpath_pointer(SLOWPATH_EVENT_MOUSE, wheel_flags(delta, horizontal, input_flags)?, x, y))
}

pub fn build_slowpath_input_pdus(share_id: u32, user_id: u16, events: &[Vec<u8>]) -> RdpResult<Vec<Vec<u8>>> {
    if events.is_empty() {
        return Err(RdpProtocolError::protocol("slowpath input event list is empty".to_string()));
    }
    let mut pdus = Vec::new();
    for event in events {
        let mut body = WriteCursor::with_capacity(event.len() + 4);
        body.write_u16_le(1);
        body.write_u16_le(0);
        body.write_slice(event);
        pdus.push(pdu::build_data_pdu(share_id, user_id, pdu::PDU_TYPE2_INPUT, body.as_slice())?);
    }
    Ok(pdus)
}

fn pointer_button(button: u8, pressed: bool, input_flags: u16) -> RdpResult<(bool, u16)> {
    match button {
        0 | 1 | 2 => {
            let mut flags = match button {
                0 => PTR_FLAGS_BUTTON1,
                1 => PTR_FLAGS_BUTTON2,
                _ => PTR_FLAGS_BUTTON3,
            };
            if pressed {
                flags |= PTR_FLAGS_DOWN;
            }
            Ok((false, flags))
        }
        3 | 4 => {
            if input_flags & INPUT_FLAG_MOUSEX == 0 {
                return Err(RdpProtocolError::unsupported("server did not negotiate extended mouse events".to_string()));
            }
            let mut flags = match button {
                3 => PTR_XFLAGS_BUTTON1,
                _ => PTR_XFLAGS_BUTTON2,
            };
            if pressed {
                flags |= PTR_XFLAGS_DOWN;
            }
            Ok((true, flags))
        }
        other => Err(RdpProtocolError::protocol(format!("unknown mouse button: {}", other))),
    }
}

fn wheel_flags(delta: i16, horizontal: bool, input_flags: u16) -> RdpResult<u16> {
    if horizontal && input_flags & INPUT_FLAG_MOUSE_HWHEEL == 0 {
        return Err(RdpProtocolError::unsupported("server did not negotiate horizontal wheel events".to_string()));
    }
    let mut flags = match horizontal {
        true => PTR_FLAGS_HWHEEL,
        false => PTR_FLAGS_WHEEL,
    };
    if delta < 0 {
        flags |= PTR_FLAGS_WHEEL_NEGATIVE;
        flags |= (0x100 + i32::from(delta)) as u16 & WHEEL_ROTATION_MASK as u16;
    } else {
        flags |= (delta as u16) & WHEEL_ROTATION_MASK as u16;
    }
    Ok(flags)
}

fn mouse_event(flags: u16, x: u16, y: u16) -> Vec<u8> {
    write_pointer_event(fastpath::INPUT_EVENT_MOUSE, flags, x, y)
}

fn mouse_extended_event(flags: u16, x: u16, y: u16) -> Vec<u8> {
    write_pointer_event(fastpath::INPUT_EVENT_MOUSEX, flags, x, y)
}

fn write_pointer_event(event_code: u8, flags: u16, x: u16, y: u16) -> Vec<u8> {
    let mut cursor = WriteCursor::with_capacity(7);
    cursor.write_u8(event_header(event_code, 0));
    cursor.write_u16_le(flags);
    cursor.write_u16_le(x);
    cursor.write_u16_le(y);
    cursor.into_vec()
}

fn event_header(event_code: u8, flags: u8) -> u8 {
    (event_code << 5) | (flags & 0x1F)
}

fn write_slowpath_pointer(message_type: u16, flags: u16, x: u16, y: u16) -> Vec<u8> {
    write_slowpath_event(message_type, |cursor| {
        cursor.write_u16_le(flags);
        cursor.write_u16_le(x);
        cursor.write_u16_le(y);
    })
}

fn write_slowpath_event(message_type: u16, write_data: impl FnOnce(&mut WriteCursor)) -> Vec<u8> {
    let mut cursor = WriteCursor::with_capacity(12);
    cursor.write_u32_le(0);
    cursor.write_u16_le(message_type);
    write_data(&mut cursor);
    cursor.into_vec()
}
