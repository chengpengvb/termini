use crate::error::SerialError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SerialParity {
    None,
    Odd,
    Even,
}

impl SerialParity {
    pub fn from_str_loose(raw: &str) -> Result<Self, SerialError> {
        match raw.trim().to_lowercase().as_str() {
            "" | "none" | "n" => Ok(SerialParity::None),
            "odd" | "o" => Ok(SerialParity::Odd),
            "even" | "e" => Ok(SerialParity::Even),
            other => Err(SerialError::InvalidConfig(format!("unknown parity: {}", other))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SerialFlowControl {
    None,
    Software,
    Hardware,
}

impl SerialFlowControl {
    pub fn from_str_loose(raw: &str) -> Result<Self, SerialError> {
        match raw.trim().to_lowercase().as_str() {
            "" | "none" => Ok(SerialFlowControl::None),
            "software" | "xonxoff" => Ok(SerialFlowControl::Software),
            "hardware" | "rtscts" => Ok(SerialFlowControl::Hardware),
            other => Err(SerialError::InvalidConfig(format!("unknown flow control: {}", other))),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SerialConfig {
    pub port: String,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub stop_bits: u8,
    pub parity: SerialParity,
    pub flow_control: SerialFlowControl,
}

impl SerialConfig {
    pub fn validate(&self) -> Result<(), SerialError> {
        if self.port.trim().is_empty() {
            return Err(SerialError::InvalidConfig("serial port name is empty".to_string()));
        }
        if self.baud_rate == 0 {
            return Err(SerialError::InvalidConfig("baud rate must not be 0".to_string()));
        }
        if !matches!(self.data_bits, 5..=8) {
            return Err(SerialError::InvalidConfig(format!("data bits must be 5-8, got {}", self.data_bits)));
        }
        if !matches!(self.stop_bits, 1 | 2) {
            return Err(SerialError::InvalidConfig(format!("stop bits must be 1 or 2, got {}", self.stop_bits)));
        }
        Ok(())
    }
}

impl Default for SerialConfig {
    fn default() -> Self {
        Self { port: String::new(), baud_rate: 9600, data_bits: 8, stop_bits: 1, parity: SerialParity::None, flow_control: SerialFlowControl::None }
    }
}
