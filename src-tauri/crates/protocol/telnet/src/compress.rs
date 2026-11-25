use crate::error::{TelnetError, TelnetProtocolError};
use flate2::{Compress, Compression, Decompress, FlushCompress, FlushDecompress, Status};

const COMPRESSION_BUFFER_SIZE: usize = 1024;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TelnetCompressionStatus {
    Enabled,
    Disabled,
}

#[derive(Debug)]
pub struct TelnetCompressor {
    inflater: Option<Decompress>,
    deflater: Option<Compress>,
}

impl TelnetCompressor {
    pub fn new() -> Self {
        Self { inflater: None, deflater: None }
    }

    pub fn is_inflating(&self) -> bool {
        self.inflater.is_some()
    }

    pub fn is_deflating(&self) -> bool {
        self.deflater.is_some()
    }

    pub fn start_inflate(&mut self) -> Result<(), TelnetProtocolError> {
        if self.inflater.is_some() || self.deflater.is_some() {
            return Err(Self::error("cannot initialize compression twice"));
        }
        self.inflater = Some(Decompress::new(true));
        Ok(())
    }

    pub fn start_deflate(&mut self) -> Result<(), TelnetProtocolError> {
        if self.inflater.is_some() || self.deflater.is_some() {
            return Err(Self::error("cannot initialize compression twice"));
        }
        self.deflater = Some(Compress::new(Compression::default(), true));
        Ok(())
    }

    pub fn inflate(&mut self, input: &[u8]) -> Result<(Vec<u8>, TelnetCompressionStatus), TelnetProtocolError> {
        let inflater = match self.inflater.as_mut() {
            Some(inflater) => inflater,
            None => return Ok((input.to_vec(), TelnetCompressionStatus::Disabled)),
        };
        let (output, status) = Self::inflate_with(inflater, input)?;
        if status == Status::StreamEnd {
            self.inflater = None;
            return Ok((output, TelnetCompressionStatus::Disabled));
        }
        Ok((output, TelnetCompressionStatus::Enabled))
    }

    pub fn deflate(&mut self, input: &[u8]) -> Result<Vec<u8>, TelnetProtocolError> {
        let deflater = match self.deflater.as_mut() {
            Some(deflater) => deflater,
            None => return Ok(input.to_vec()),
        };
        Self::deflate_with(deflater, input)
    }

    fn inflate_with(inflater: &mut Decompress, input: &[u8]) -> Result<(Vec<u8>, Status), TelnetProtocolError> {
        let mut output = Vec::new();
        let mut input_offset = 0;
        loop {
            let mut buffer = [0; COMPRESSION_BUFFER_SIZE];
            let before_in = inflater.total_in();
            let before_out = inflater.total_out();
            let status = inflater.decompress(&input[input_offset..], &mut buffer, FlushDecompress::Sync).map_err(|error| Self::error(&format!("inflate() failed: {}", error)))?;
            let consumed = (inflater.total_in() - before_in) as usize;
            let produced = (inflater.total_out() - before_out) as usize;
            output.extend_from_slice(&buffer[..produced]);
            input_offset += consumed;
            if status == Status::StreamEnd || Self::is_stream_drained(input, input_offset, consumed, produced) {
                return Ok((output, status));
            }
        }
    }

    fn deflate_with(deflater: &mut Compress, input: &[u8]) -> Result<Vec<u8>, TelnetProtocolError> {
        let mut output = Vec::new();
        let mut input_offset = 0;
        loop {
            let mut buffer = [0; COMPRESSION_BUFFER_SIZE];
            let before_in = deflater.total_in();
            let before_out = deflater.total_out();
            deflater.compress(&input[input_offset..], &mut buffer, FlushCompress::Sync).map_err(|error| Self::error(&format!("deflate() failed: {}", error)))?;
            let consumed = (deflater.total_in() - before_in) as usize;
            let produced = (deflater.total_out() - before_out) as usize;
            output.extend_from_slice(&buffer[..produced]);
            input_offset += consumed;
            if Self::is_stream_drained(input, input_offset, consumed, produced) {
                return Ok(output);
            }
        }
    }

    fn is_stream_drained(input: &[u8], input_offset: usize, consumed: usize, produced: usize) -> bool {
        let output_has_capacity = produced < COMPRESSION_BUFFER_SIZE;
        let input_consumed = input_offset >= input.len();
        input_consumed && output_has_capacity || consumed == 0 && produced == 0
    }

    fn error(message: &str) -> TelnetProtocolError {
        TelnetProtocolError::new(TelnetError::Compress, message.to_string())
    }
}

impl Default for TelnetCompressor {
    fn default() -> Self {
        Self::new()
    }
}
