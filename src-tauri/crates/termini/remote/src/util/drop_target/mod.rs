use anyhow::Result;
use crate::model::resp::resp_sftp::RespSftpRemoteDrag;
use crate::web::ws::handle::r#impl::handle_sftp::HandleSftp;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

#[cfg(any(target_os = "macos", windows, target_os = "linux"))]
pub fn start_remote_drag(handle: tauri::AppHandle, sftp_handler: HandleSftp, req: RespSftpRemoteDrag) -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::start_remote_drag(handle, sftp_handler, req);

    #[cfg(windows)]
    return windows::start_remote_drag(handle, sftp_handler, req);

    #[cfg(target_os = "linux")]
    return linux::start_remote_drag(handle, sftp_handler, req);
}

#[cfg(not(any(target_os = "macos", windows, target_os = "linux")))]
pub fn start_remote_drag(_: tauri::AppHandle, _: HandleSftp, _: RespSftpRemoteDrag) -> Result<()> {
    Err(anyhow::anyhow!("remote OS drag not supported on this platform"))
}