use crate::core::error::{RdpProtocolError, RdpResult};

pub const TAG_INTEGER: u8 = 0x02;
pub const TAG_BIT_STRING: u8 = 0x03;
pub const TAG_OCTET_STRING: u8 = 0x04;
pub const TAG_SEQUENCE: u8 = 0x30;

pub const CLASS_CONTEXT_CONSTRUCTED: u8 = 0xA0;

pub fn context_tag(index: u8) -> u8 {
    CLASS_CONTEXT_CONSTRUCTED | index
}

pub fn encode_length(length: usize) -> Vec<u8> {
    if length < 0x80 {
        return vec![length as u8];
    }
    let bytes = length.to_be_bytes();
    let start = bytes.iter().position(|byte| *byte != 0).unwrap_or(bytes.len() - 1);
    let significant = &bytes[start..];
    let mut encoded = Vec::with_capacity(significant.len() + 1);
    encoded.push(0x80 | significant.len() as u8);
    encoded.extend_from_slice(significant);
    encoded
}

pub fn encode_tlv(tag: u8, content: &[u8]) -> Vec<u8> {
    let length = encode_length(content.len());
    let mut encoded = Vec::with_capacity(1 + length.len() + content.len());
    encoded.push(tag);
    encoded.extend_from_slice(&length);
    encoded.extend_from_slice(content);
    encoded
}

pub fn encode_sequence(content: &[u8]) -> Vec<u8> {
    encode_tlv(TAG_SEQUENCE, content)
}

pub fn encode_octet_string(content: &[u8]) -> Vec<u8> {
    encode_tlv(TAG_OCTET_STRING, content)
}

pub fn encode_context(index: u8, content: &[u8]) -> Vec<u8> {
    encode_tlv(context_tag(index), content)
}

pub fn encode_integer_u32(value: u32) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let start = bytes.iter().position(|byte| *byte != 0).unwrap_or(bytes.len() - 1);
    let mut content = Vec::with_capacity(5);
    if bytes[start] & 0x80 != 0 {
        content.push(0);
    }
    content.extend_from_slice(&bytes[start..]);
    encode_tlv(TAG_INTEGER, &content)
}

#[derive(Debug, Clone, Copy)]
pub struct DerElement<'a> {
    pub tag: u8,
    pub content: &'a [u8],
    pub raw: &'a [u8],
}

pub struct DerReader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> DerReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.position >= self.data.len()
    }

    pub fn peek_tag(&self) -> Option<u8> {
        self.data.get(self.position).copied()
    }

    pub fn next_element(&mut self) -> RdpResult<DerElement<'a>> {
        let start = self.position;
        let tag = self.take(1)?[0];
        if tag & 0x1F == 0x1F {
            return Err(RdpProtocolError::encoding("der multi-byte tag not supported".to_string()));
        }
        let first = self.take(1)?[0];
        let length = if first < 0x80 {
            first as usize
        } else {
            let count = (first & 0x7F) as usize;
            if count == 0 || count > 4 {
                return Err(RdpProtocolError::encoding(format!("der length octet count invalid: {}", count)));
            }
            let bytes = self.take(count)?;
            bytes.iter().fold(0usize, |acc, byte| (acc << 8) | *byte as usize)
        };
        let content = self.take(length)?;
        Ok(DerElement { tag, content, raw: &self.data[start..self.position] })
    }

    pub fn expect(&mut self, tag: u8) -> RdpResult<DerElement<'a>> {
        let element = self.next_element()?;
        if element.tag != tag {
            return Err(RdpProtocolError::encoding(format!("der expected tag 0x{:02x}, got 0x{:02x}", tag, element.tag)));
        }
        Ok(element)
    }

    fn take(&mut self, count: usize) -> RdpResult<&'a [u8]> {
        if self.data.len() - self.position < count {
            return Err(RdpProtocolError::underrun(format!("der need {} bytes, have {}", count, self.data.len() - self.position)));
        }
        let start = self.position;
        self.position += count;
        Ok(&self.data[start..self.position])
    }
}

pub fn read_integer_u32(content: &[u8]) -> RdpResult<u32> {
    if content.is_empty() || content.len() > 5 {
        return Err(RdpProtocolError::encoding(format!("der integer length invalid: {}", content.len())));
    }
    let mut value: u32 = 0;
    for byte in content.iter().skip_while(|byte| **byte == 0) {
        value = value.checked_shl(8).ok_or_else(|| RdpProtocolError::encoding("der integer overflow".to_string()))? | *byte as u32;
    }
    Ok(value)
}
