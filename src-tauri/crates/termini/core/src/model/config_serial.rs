use termini_macros::Redact;
use termini_macros::base_target;
use serde::{Deserialize, Serialize};

pub const SERIAL_BAUD_RATE_DEFAULT: i32 = 9600;
pub const SERIAL_DATA_BITS_DEFAULT: i16 = 8;
pub const SERIAL_STOP_BITS_DEFAULT: i16 = 1;
pub const SERIAL_PARITY_DEFAULT: &str = "none";
pub const SERIAL_FLOW_CONTROL_DEFAULT: &str = "none";

#[base_target(table = "config_serial", entity_type = 17, notify = 1)]
#[derive(Clone, Serialize, Deserialize, Redact)]
pub struct Model {
    pub port_name: String,
    pub baud_rate: i32,
    pub data_bits: i16,
    pub stop_bits: i16,
    pub parity: String,
    pub flow_control: String,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            id: String::new(),
            port_name: String::new(),
            baud_rate: SERIAL_BAUD_RATE_DEFAULT,
            data_bits: SERIAL_DATA_BITS_DEFAULT,
            stop_bits: SERIAL_STOP_BITS_DEFAULT,
            parity: SERIAL_PARITY_DEFAULT.to_string(),
            flow_control: SERIAL_FLOW_CONTROL_DEFAULT.to_string(),
        }
    }
}

pub type ConfigSerial = Model;
