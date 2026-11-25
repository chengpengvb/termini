use crate::core::error::{RdpProtocolError, RdpResult};

pub struct ReadCursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> ReadCursor<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, position: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.position
    }

    pub fn rest(&self) -> &'a [u8] {
        &self.data[self.position..]
    }

    pub fn ensure(&self, count: usize) -> RdpResult<()> {
        if self.remaining() < count {
            return Err(RdpProtocolError::underrun(format!("need {} bytes, have {}", count, self.remaining())));
        }
        Ok(())
    }

    pub fn skip(&mut self, count: usize) -> RdpResult<()> {
        self.ensure(count)?;
        self.position += count;
        Ok(())
    }

    pub fn read_u8(&mut self) -> RdpResult<u8> {
        self.ensure(1)?;
        let value = self.data[self.position];
        self.position += 1;
        Ok(value)
    }

    pub fn peek_u8(&self) -> RdpResult<u8> {
        self.ensure(1)?;
        Ok(self.data[self.position])
    }

    pub fn read_u16_le(&mut self) -> RdpResult<u16> {
        let bytes = self.read_slice(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u16_be(&mut self) -> RdpResult<u16> {
        let bytes = self.read_slice(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    pub fn read_u32_le(&mut self) -> RdpResult<u32> {
        let bytes = self.read_slice(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_u32_be(&mut self) -> RdpResult<u32> {
        let bytes = self.read_slice(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    pub fn read_slice(&mut self, count: usize) -> RdpResult<&'a [u8]> {
        self.ensure(count)?;
        let start = self.position;
        self.position += count;
        Ok(&self.data[start..self.position])
    }

    pub fn read_vec(&mut self, count: usize) -> RdpResult<Vec<u8>> {
        Ok(self.read_slice(count)?.to_vec())
    }

    pub fn sub_cursor(&mut self, count: usize) -> RdpResult<ReadCursor<'a>> {
        Ok(ReadCursor::new(self.read_slice(count)?))
    }
}

#[derive(Default)]
pub struct WriteCursor {
    data: Vec<u8>,
}

impl WriteCursor {
    pub fn with_capacity(capacity: usize) -> Self {
        Self { data: Vec::with_capacity(capacity) }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }

    pub fn write_u8(&mut self, value: u8) {
        self.data.push(value);
    }

    pub fn write_u16_le(&mut self, value: u16) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u16_be(&mut self, value: u16) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_u32_le(&mut self, value: u32) {
        self.data.extend_from_slice(&value.to_le_bytes());
    }

    pub fn write_u32_be(&mut self, value: u32) {
        self.data.extend_from_slice(&value.to_be_bytes());
    }

    pub fn write_slice(&mut self, value: &[u8]) {
        self.data.extend_from_slice(value);
    }

    pub fn write_zeros(&mut self, count: usize) {
        self.data.resize(self.data.len() + count, 0);
    }

    pub fn write_utf16_le(&mut self, value: &str) {
        for unit in value.encode_utf16() {
            self.write_u16_le(unit);
        }
    }

    pub fn write_utf16_le_padded(&mut self, value: &str, byte_len: usize) -> RdpResult<()> {
        let start = self.data.len();
        self.write_utf16_le(value);
        let written = self.data.len() - start;
        if written > byte_len {
            return Err(RdpProtocolError::encoding(format!("utf16 field overflow: {} > {}", written, byte_len)));
        }
        self.write_zeros(byte_len - written);
        Ok(())
    }

    pub fn patch_u8(&mut self, offset: usize, value: u8) -> RdpResult<()> {
        self.patch(offset, &[value])
    }

    pub fn patch_u16_le(&mut self, offset: usize, value: u16) -> RdpResult<()> {
        self.patch(offset, &value.to_le_bytes())
    }

    pub fn patch_u16_be(&mut self, offset: usize, value: u16) -> RdpResult<()> {
        self.patch(offset, &value.to_be_bytes())
    }

    fn patch(&mut self, offset: usize, bytes: &[u8]) -> RdpResult<()> {
        if offset + bytes.len() > self.data.len() {
            return Err(RdpProtocolError::encoding(format!("patch {}+{} beyond {}", offset, bytes.len(), self.data.len())));
        }
        self.data[offset..offset + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }
}
