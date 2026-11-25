use crate::core::stream::{ReadCursor, WriteCursor};
use crate::core::error::{RdpProtocolError, RdpResult};

pub fn write_length(cursor: &mut WriteCursor, length: u16) -> RdpResult<()> {
    if length > 0x7FFF {
        return Err(RdpProtocolError::encoding(format!("per length overflow: {}", length)));
    }
    if length > 0x7F {
        cursor.write_u16_be(length | 0x8000);
        return Ok(());
    }
    cursor.write_u8(length as u8);
    Ok(())
}

pub fn read_length(cursor: &mut ReadCursor<'_>) -> RdpResult<u16> {
    let first = cursor.read_u8()?;
    if first & 0x80 != 0 {
        let second = cursor.read_u8()?;
        return Ok((((first & 0x7F) as u16) << 8) | second as u16);
    }
    Ok(first as u16)
}

pub fn write_choice(cursor: &mut WriteCursor, choice: u8) {
    cursor.write_u8(choice);
}

pub fn read_choice(cursor: &mut ReadCursor<'_>) -> RdpResult<u8> {
    cursor.read_u8()
}

pub fn write_selection(cursor: &mut WriteCursor, selection: u8) {
    cursor.write_u8(selection);
}

pub fn write_number_of_sets(cursor: &mut WriteCursor, count: u8) {
    cursor.write_u8(count);
}

pub fn read_number_of_sets(cursor: &mut ReadCursor<'_>) -> RdpResult<u8> {
    cursor.read_u8()
}

pub fn write_padding(cursor: &mut WriteCursor, length: usize) {
    cursor.write_zeros(length);
}

pub fn write_integer(cursor: &mut WriteCursor, value: u32) {
    if value <= 0xFF {
        cursor.write_u8(1);
        cursor.write_u8(value as u8);
        return;
    }
    if value <= 0xFFFF {
        cursor.write_u8(2);
        cursor.write_u16_be(value as u16);
        return;
    }
    cursor.write_u8(4);
    cursor.write_u32_be(value);
}

pub fn read_integer(cursor: &mut ReadCursor<'_>) -> RdpResult<u32> {
    let length = cursor.read_u8()?;
    match length {
        1 => Ok(cursor.read_u8()? as u32),
        2 => Ok(cursor.read_u16_be()? as u32),
        4 => cursor.read_u32_be(),
        _ => Err(RdpProtocolError::encoding(format!("per integer length invalid: {}", length))),
    }
}

pub fn write_integer16(cursor: &mut WriteCursor, value: u16, min: u16) -> RdpResult<()> {
    if value < min {
        return Err(RdpProtocolError::encoding(format!("per integer16 below minimum: {} < {}", value, min)));
    }
    cursor.write_u16_be(value - min);
    Ok(())
}

pub fn read_integer16(cursor: &mut ReadCursor<'_>, min: u16) -> RdpResult<u16> {
    let value = cursor.read_u16_be()?;
    value.checked_add(min).ok_or_else(|| RdpProtocolError::encoding("per integer16 overflow".to_string()))
}

pub fn read_enumerated(cursor: &mut ReadCursor<'_>, count: u8) -> RdpResult<u8> {
    let value = cursor.read_u8()?;
    if value >= count {
        return Err(RdpProtocolError::encoding(format!("per enumerated out of range: {} >= {}", value, count)));
    }
    Ok(value)
}

pub fn write_object_identifier(cursor: &mut WriteCursor, oid: &[u8; 6]) {
    cursor.write_u8(5);
    cursor.write_u8((oid[0] << 4) | (oid[1] & 0x0F));
    cursor.write_u8(oid[2]);
    cursor.write_u8(oid[3]);
    cursor.write_u8(oid[4]);
    cursor.write_u8(oid[5]);
}

pub fn read_object_identifier(cursor: &mut ReadCursor<'_>, expected: &[u8; 6]) -> RdpResult<()> {
    let length = cursor.read_u8()?;
    if length != 5 {
        return Err(RdpProtocolError::encoding(format!("per oid length invalid: {}", length)));
    }
    let bytes = cursor.read_slice(5)?;
    let actual = [bytes[0] >> 4, bytes[0] & 0x0F, bytes[1], bytes[2], bytes[3], bytes[4]];
    if &actual != expected {
        return Err(RdpProtocolError::encoding(format!("per oid mismatch: {:?}", actual)));
    }
    Ok(())
}

pub fn write_octet_string(cursor: &mut WriteCursor, value: &[u8], min: usize) -> RdpResult<()> {
    let length = if value.len() >= min { value.len() - min } else { min };
    write_length(cursor, u16::try_from(length).map_err(|_| RdpProtocolError::encoding("per octet string too long".to_string()))?)?;
    cursor.write_slice(value);
    Ok(())
}

pub fn read_octet_string<'a>(cursor: &mut ReadCursor<'a>, min: usize) -> RdpResult<&'a [u8]> {
    let length = read_length(cursor)? as usize + min;
    cursor.read_slice(length)
}

pub fn write_numeric_string(cursor: &mut WriteCursor, value: &[u8], min: usize) -> RdpResult<()> {
    let length = if value.len() >= min { value.len() - min } else { min };
    write_length(cursor, u16::try_from(length).map_err(|_| RdpProtocolError::encoding("per numeric string too long".to_string()))?)?;
    let mut index = 0;
    while index < value.len() {
        let high = value[index].wrapping_sub(b'0');
        let low = if index + 1 < value.len() { value[index + 1].wrapping_sub(b'0') } else { 0 };
        cursor.write_u8((high << 4) | low);
        index += 2;
    }
    Ok(())
}
