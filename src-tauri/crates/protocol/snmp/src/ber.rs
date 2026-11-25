use crate::value::Oid;
use crate::error::SnmpError;

pub const TAG_INTEGER: u8 = 0x02;
pub const TAG_OCTET_STRING: u8 = 0x04;
pub const TAG_NULL: u8 = 0x05;
pub const TAG_OID: u8 = 0x06;
pub const TAG_SEQUENCE: u8 = 0x30;
pub const TAG_IP_ADDRESS: u8 = 0x40;
pub const TAG_COUNTER32: u8 = 0x41;
pub const TAG_GAUGE32: u8 = 0x42;
pub const TAG_TIME_TICKS: u8 = 0x43;
pub const TAG_OPAQUE: u8 = 0x44;
pub const TAG_COUNTER64: u8 = 0x46;
pub const TAG_NO_SUCH_OBJECT: u8 = 0x80;
pub const TAG_NO_SUCH_INSTANCE: u8 = 0x81;
pub const TAG_END_OF_MIB_VIEW: u8 = 0x82;

pub struct BerWriter {
    buf: Vec<u8>,
}

impl BerWriter {
    pub fn new() -> Self {
        BerWriter { buf: Vec::with_capacity(128) }
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    pub fn write_integer(&mut self, value: i64) {
        let mut bytes = value.to_be_bytes().to_vec();
        while bytes.len() > 1 {
            let first = bytes[0];
            let second_high = bytes[1] & 0x80;
            if (first == 0x00 && second_high == 0) || (first == 0xFF && second_high != 0) {
                bytes.remove(0);
            } else {
                break;
            }
        }
        self.write_tlv(TAG_INTEGER, &bytes);
    }

    pub fn write_unsigned(&mut self, tag: u8, value: u64) {
        let mut bytes = value.to_be_bytes().to_vec();
        while bytes.len() > 1 && bytes[0] == 0x00 && bytes[1] & 0x80 == 0 {
            bytes.remove(0);
        }
        if bytes[0] & 0x80 != 0 {
            bytes.insert(0, 0x00);
        }
        self.write_tlv(tag, &bytes);
    }

    pub fn write_octet_string(&mut self, bytes: &[u8]) {
        self.write_tlv(TAG_OCTET_STRING, bytes);
    }

    pub fn write_null(&mut self) {
        self.write_tlv(TAG_NULL, &[]);
    }

    pub fn write_oid(&mut self, oid: &Oid) {
        let mut content = Vec::with_capacity(oid.0.len() + 4);
        let first = oid.0.first().copied().unwrap_or(1);
        let second = oid.0.get(1).copied().unwrap_or(3);
        content.push((first * 40 + second) as u8);
        for arc in oid.0.iter().skip(2) {
            Self::push_base128(&mut content, *arc);
        }
        self.write_tlv(TAG_OID, &content);
    }

    pub fn write_tlv(&mut self, tag: u8, content: &[u8]) {
        self.buf.push(tag);
        Self::push_length(&mut self.buf, content.len());
        self.buf.extend_from_slice(content);
    }

    pub fn write_wrapped(&mut self, tag: u8, inner: BerWriter) {
        let content = inner.into_bytes();
        self.write_tlv(tag, &content);
    }

    fn push_length(buf: &mut Vec<u8>, len: usize) {
        if len < 0x80 {
            buf.push(len as u8);
            return;
        }
        let bytes = (len as u64).to_be_bytes();
        let skip = bytes.iter().take_while(|byte| **byte == 0).count();
        let significant = &bytes[skip..];
        buf.push(0x80 | significant.len() as u8);
        buf.extend_from_slice(significant);
    }

    fn push_base128(buf: &mut Vec<u8>, value: u32) {
        if value < 0x80 {
            buf.push(value as u8);
            return;
        }
        let mut chunks = Vec::with_capacity(5);
        let mut rest = value;
        while rest > 0 {
            chunks.push((rest & 0x7F) as u8);
            rest >>= 7;
        }
        for (index, chunk) in chunks.iter().rev().enumerate() {
            if index + 1 == chunks.len() {
                buf.push(*chunk);
            } else {
                buf.push(*chunk | 0x80);
            }
        }
    }
}

pub struct BerReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> BerReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        BerReader { data, pos: 0 }
    }

    pub fn has_remaining(&self) -> bool {
        self.pos < self.data.len()
    }

    pub fn peek_tag(&self) -> Result<u8, SnmpError> {
        self.data.get(self.pos).copied().ok_or_else(|| SnmpError::Decode("insufficient data: reading tag".to_string()))
    }

    pub fn read_tlv(&mut self) -> Result<(u8, &'a [u8]), SnmpError> {
        let tag = self.peek_tag()?;
        self.pos += 1;
        let len = self.read_length()?;
        if self.pos + len > self.data.len() {
            return Err(SnmpError::Decode(format!("insufficient data: declared length {} exceeds remaining bytes", len)));
        }
        let content = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok((tag, content))
    }

    pub fn expect_tlv(&mut self, expected: u8) -> Result<&'a [u8], SnmpError> {
        let (tag, content) = self.read_tlv()?;
        if tag != expected {
            return Err(SnmpError::Decode(format!("expected tag 0x{:02X}, got 0x{:02X}", expected, tag)));
        }
        Ok(content)
    }

    pub fn read_integer(&mut self) -> Result<i64, SnmpError> {
        let content = self.expect_tlv(TAG_INTEGER)?;
        Ok(Self::decode_signed(content))
    }

    fn read_length(&mut self) -> Result<usize, SnmpError> {
        let first = self.data.get(self.pos).copied().ok_or_else(|| SnmpError::Decode("insufficient data: reading length".to_string()))?;
        self.pos += 1;
        if first & 0x80 == 0 {
            return Ok(first as usize);
        }
        let count = (first & 0x7F) as usize;
        if count == 0 || count > 4 {
            return Err(SnmpError::Decode(format!("unsupported length octet count: {}", count)));
        }
        if self.pos + count > self.data.len() {
            return Err(SnmpError::Decode("insufficient data: reading long-form length".to_string()));
        }
        let mut len = 0usize;
        for _ in 0..count {
            len = (len << 8) | self.data[self.pos] as usize;
            self.pos += 1;
        }
        Ok(len)
    }

    pub fn decode_signed(content: &[u8]) -> i64 {
        if content.is_empty() {
            return 0;
        }
        let mut value = if content[0] & 0x80 != 0 { -1i64 } else { 0i64 };
        for byte in content {
            value = (value << 8) | *byte as i64;
        }
        value
    }

    pub fn decode_unsigned(content: &[u8]) -> u64 {
        let mut value = 0u64;
        for byte in content {
            value = (value << 8) | *byte as u64;
        }
        value
    }

    pub fn decode_oid(content: &[u8]) -> Result<Oid, SnmpError> {
        if content.is_empty() {
            return Err(SnmpError::Decode("OID content is empty".to_string()));
        }
        let mut arcs = Vec::with_capacity(content.len() + 1);
        let first = content[0] as u32;
        arcs.push(first / 40);
        arcs.push(first % 40);
        let mut current = 0u32;
        for byte in &content[1..] {
            current = (current << 7) | (*byte & 0x7F) as u32;
            if byte & 0x80 == 0 {
                arcs.push(current);
                current = 0;
            }
        }
        Ok(Oid(arcs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn integer_roundtrip() {
        for value in [0i64, 1, 127, 128, 255, 256, -1, -128, -129, 2147483647, -2147483648] {
            let mut writer = BerWriter::new();
            writer.write_integer(value);
            let bytes = writer.into_bytes();
            let mut reader = BerReader::new(&bytes);
            assert_eq!(reader.read_integer().unwrap(), value, "value={}", value);
        }
    }

    #[test]
    fn unsigned_high_bit_prefixed() {
        let mut writer = BerWriter::new();
        writer.write_unsigned(TAG_COUNTER32, 4294967295);
        let bytes = writer.into_bytes();
        assert_eq!(bytes, vec![0x41, 0x05, 0x00, 0xFF, 0xFF, 0xFF, 0xFF]);
        let mut reader = BerReader::new(&bytes);
        let (tag, content) = reader.read_tlv().unwrap();
        assert_eq!(tag, TAG_COUNTER32);
        assert_eq!(BerReader::decode_unsigned(content), 4294967295);
    }

    #[test]
    fn oid_roundtrip() {
        let oid = Oid::from_str("1.3.6.1.4.1.2021.11.9.0").unwrap();
        let mut writer = BerWriter::new();
        writer.write_oid(&oid);
        let bytes = writer.into_bytes();
        let mut reader = BerReader::new(&bytes);
        let content = reader.expect_tlv(TAG_OID).unwrap();
        assert_eq!(BerReader::decode_oid(content).unwrap(), oid);
    }

    #[test]
    fn oid_large_arc() {
        let oid = Oid::from_str("1.3.6.1.4.1.311.1.1.3.1.2").unwrap();
        let mut writer = BerWriter::new();
        writer.write_oid(&oid);
        let bytes = writer.into_bytes();
        let mut reader = BerReader::new(&bytes);
        let content = reader.expect_tlv(TAG_OID).unwrap();
        assert_eq!(BerReader::decode_oid(content).unwrap(), oid);
    }

    #[test]
    fn long_form_length() {
        let content = vec![0xAB; 300];
        let mut writer = BerWriter::new();
        writer.write_octet_string(&content);
        let bytes = writer.into_bytes();
        assert_eq!(&bytes[..4], &[0x04, 0x82, 0x01, 0x2C]);
        let mut reader = BerReader::new(&bytes);
        let decoded = reader.expect_tlv(TAG_OCTET_STRING).unwrap();
        assert_eq!(decoded, content.as_slice());
    }
}
