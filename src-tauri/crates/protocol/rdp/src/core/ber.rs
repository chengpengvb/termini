use crate::core::error::{RdpProtocolError, RdpResult};

pub const CLASS_UNIVERSAL: u8 = 0x00;
pub const CLASS_APPLICATION: u8 = 0x40;
pub const PC_CONSTRUCTED: u8 = 0x20;

pub const TAG_BOOLEAN: u8 = 0x01;
pub const TAG_INTEGER: u8 = 0x02;
pub const TAG_OCTET_STRING: u8 = 0x04;
pub const TAG_ENUMERATED: u8 = 0x0A;
pub const TAG_SEQUENCE: u8 = 0x10;
pub const TAG_MASK: u8 = 0x1F;

pub fn encode_length(length: usize) -> Vec<u8> {
    if length < 0x80 {
        return vec![length as u8];
    }
    if length <= 0xFF {
        return vec![0x81, length as u8];
    }
    if length <= 0xFFFF {
        let bytes = (length as u16).to_be_bytes();
        return vec![0x82, bytes[0], bytes[1]];
    }
    let bytes = (length as u32).to_be_bytes();
    vec![0x84, bytes[0], bytes[1], bytes[2], bytes[3]]
}

pub fn encode_universal(tag: u8, constructed: bool, content: &[u8]) -> Vec<u8> {
    let identifier = CLASS_UNIVERSAL | if constructed { PC_CONSTRUCTED } else { 0 } | tag;
    encode_identified(&[identifier], content)
}

pub fn encode_application(tag: u8, content: &[u8]) -> Vec<u8> {
    if tag > 30 {
        return encode_identified(&[CLASS_APPLICATION | PC_CONSTRUCTED | TAG_MASK, tag], content);
    }
    encode_identified(&[CLASS_APPLICATION | PC_CONSTRUCTED | tag], content)
}

pub fn encode_sequence(content: &[u8]) -> Vec<u8> {
    encode_universal(TAG_SEQUENCE, true, content)
}

pub fn encode_octet_string(content: &[u8]) -> Vec<u8> {
    encode_universal(TAG_OCTET_STRING, false, content)
}

pub fn encode_boolean(value: bool) -> Vec<u8> {
    encode_universal(TAG_BOOLEAN, false, &[if value { 0xFF } else { 0x00 }])
}

pub fn encode_integer(value: u32) -> Vec<u8> {
    let content = if value <= 0xFF {
        vec![value as u8]
    } else if value <= 0xFFFF {
        let bytes = (value as u16).to_be_bytes();
        vec![0x00, bytes[0], bytes[1]]
    } else {
        let bytes = value.to_be_bytes();
        vec![bytes[0], bytes[1], bytes[2], bytes[3]]
    };
    encode_universal(TAG_INTEGER, false, &content)
}

fn encode_identified(identifier: &[u8], content: &[u8]) -> Vec<u8> {
    let length = encode_length(content.len());
    let mut encoded = Vec::with_capacity(identifier.len() + length.len() + content.len());
    encoded.extend_from_slice(identifier);
    encoded.extend_from_slice(&length);
    encoded.extend_from_slice(content);
    encoded
}

pub struct BerReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> BerReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    pub fn read_universal(&mut self, tag: u8, constructed: bool) -> RdpResult<&'a [u8]> {
        let identifier = CLASS_UNIVERSAL | if constructed { PC_CONSTRUCTED } else { 0 } | tag;
        self.read_identified(&[identifier])
    }

    pub fn read_application(&mut self, tag: u8) -> RdpResult<&'a [u8]> {
        if tag > 30 {
            return self.read_identified(&[CLASS_APPLICATION | PC_CONSTRUCTED | TAG_MASK, tag]);
        }
        self.read_identified(&[CLASS_APPLICATION | PC_CONSTRUCTED | tag])
    }

    pub fn read_sequence(&mut self) -> RdpResult<&'a [u8]> {
        self.read_universal(TAG_SEQUENCE, true)
    }

    pub fn read_octet_string(&mut self) -> RdpResult<&'a [u8]> {
        self.read_universal(TAG_OCTET_STRING, false)
    }

    pub fn read_integer(&mut self) -> RdpResult<u32> {
        let content = self.read_universal(TAG_INTEGER, false)?;
        if content.is_empty() || content.len() > 4 {
            return Err(RdpProtocolError::encoding(format!("ber integer length invalid: {}", content.len())));
        }
        Ok(content.iter().fold(0u32, |acc, byte| (acc << 8) | *byte as u32))
    }

    pub fn read_enumerated(&mut self) -> RdpResult<u8> {
        let content = self.read_universal(TAG_ENUMERATED, false)?;
        if content.len() != 1 {
            return Err(RdpProtocolError::encoding(format!("ber enumerated length invalid: {}", content.len())));
        }
        Ok(content[0])
    }

    fn read_identified(&mut self, identifier: &[u8]) -> RdpResult<&'a [u8]> {
        let actual = self.take(identifier.len())?;
        if actual != identifier {
            return Err(RdpProtocolError::encoding(format!("ber expected identifier {:02x?}, got {:02x?}", identifier, actual)));
        }
        let length = self.read_length()?;
        self.take(length)
    }

    fn read_length(&mut self) -> RdpResult<usize> {
        let first = self.take(1)?[0];
        if first < 0x80 {
            return Ok(first as usize);
        }
        let count = (first & 0x7F) as usize;
        if count == 0 || count > 4 {
            return Err(RdpProtocolError::encoding(format!("ber length octet count invalid: {}", count)));
        }
        let bytes = self.take(count)?;
        Ok(bytes.iter().fold(0usize, |acc, byte| (acc << 8) | *byte as usize))
    }

    fn take(&mut self, count: usize) -> RdpResult<&'a [u8]> {
        if self.data.len() - self.position < count {
            return Err(RdpProtocolError::underrun(format!("ber need {} bytes, have {}", count, self.data.len() - self.position)));
        }
        let start = self.position;
        self.position += count;
        Ok(&self.data[start..self.position])
    }
}
