pub mod shell;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod serial;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod open_here;
pub use termini_core::common;