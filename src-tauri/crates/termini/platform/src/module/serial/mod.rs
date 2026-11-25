#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod client_serial;

pub use protocol_serial::available_ports;
