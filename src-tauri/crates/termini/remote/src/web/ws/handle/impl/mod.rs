pub mod sftp;
pub mod handle_rdp;
pub mod handle_ssh;
pub mod handle_sftp;
pub mod handle_shell;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod handle_serial;
pub mod handle_telnet;
pub mod handle_ssh_auth;