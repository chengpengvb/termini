pub mod error;
pub mod config;

pub use error::SerialError;
pub use config::{SerialConfig, SerialFlowControl, SerialParity};

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod session;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use session::SerialSession;

pub fn available_ports() -> Vec<String> {
    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
    {
        serial2::SerialPort::available_ports().map(|ports| ports.into_iter().map(|path| path.to_string_lossy().to_string()).collect()).unwrap_or_default()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Vec::new()
    }
}