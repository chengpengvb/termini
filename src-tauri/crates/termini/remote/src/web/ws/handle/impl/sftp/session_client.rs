use std::pin::Pin;
use anyhow::Result;
use std::sync::Arc;
use crate::util::path_util;
use tokio::io::{AsyncRead, AsyncWrite};
use crate::model::dto::cancel_session::CancelNotify;
use crate::module::sftp::client_transfer::ClientTransfer;
use crate::model::dto::resolved_list_dir::ResolvedListDir;
use crate::module::common::dto::file_entry::PlannedDelete;
use crate::module::common::context::context_ftp::ContextFtp;
use crate::module::common::context::context_ssh::ContextSsh;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::module::common::dto::file_entry::{FileEntry, FileKind};
use crate::module::sftp::client_transfer_local::ClientTransferLocal;
use crate::module::common::dto::file_stat_properties::FileStatProperties;
use crate::model::resp::resp_sftp::{RespSftpDeleteProgressHandler, RespSftpTransferProgressHandler};

pub type ClientTransferSftpDyn = dyn ClientTransfer<Context = ContextSsh, Event = RespEventSftp>;
pub type ClientTransferFtpDyn = dyn ClientTransfer<Context = ContextFtp, Event = RespEventSftp>;

#[derive(Clone)]
pub enum RemoteSessionClient {
    Sftp(Arc<ClientTransferSftpDyn>),
    Ftp(Arc<ClientTransferFtpDyn>),
}

impl RemoteSessionClient {
    pub fn get_id(&self) -> String {
        match self {
            Self::Sftp(c) => c.get_id(),
            Self::Ftp(c) => c.get_id(),
        }
    }

    pub async fn close(&self) -> Result<()> {
        match self {
            Self::Sftp(c) => c.close().await,
            Self::Ftp(c) => c.close().await,
        }
    }

    pub async fn home_dir(&self) -> Result<String> {
        match self {
            Self::Sftp(c) => c.home_dir().await,
            Self::Ftp(c) => c.home_dir().await,
        }
    }

    pub fn parent_path(&self, path: &str) -> Option<String> {
        match self {
            Self::Sftp(c) => c.parent_path(path),
            Self::Ftp(c) => c.parent_path(path),
        }
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>> {
        match self {
            Self::Sftp(c) => c.list_dir(path).await,
            Self::Ftp(c) => c.list_dir(path).await,
        }
    }

    pub async fn list_dir_for_transfer_plan(&self, path: &str, cancel: Option<&CancelNotify<'_>>) -> Result<Vec<FileEntry>> {
        match self {
            Self::Sftp(c) => c.list_dir_for_transfer_plan(path, cancel).await,
            Self::Ftp(c) => c.list_dir_for_transfer_plan(path, cancel).await,
        }
    }

    pub async fn stat(&self, path: &str, kind: &FileKind) -> Result<FileEntry> {
        match self {
            Self::Sftp(c) => c.stat(path, kind).await,
            Self::Ftp(c) => c.stat(path, kind).await,
        }
    }

    pub async fn stat_properties(&self, path: &str, kind: &FileKind) -> Result<FileStatProperties> {
        match self {
            Self::Sftp(c) => c.stat_properties(path, kind).await,
            Self::Ftp(c) => c.stat_properties(path, kind).await,
        }
    }

    pub async fn exists(&self, path: &str) -> Result<bool> {
        match self {
            Self::Sftp(c) => c.exists(path).await,
            Self::Ftp(c) => c.exists(path).await,
        }
    }

    pub async fn realpath(&self, path: &str) -> Result<String> {
        match self {
            Self::Sftp(c) => c.realpath(path).await,
            Self::Ftp(c) => c.realpath(path).await,
        }
    }

    pub async fn read_link(&self, path: &str) -> Result<String> {
        match self {
            Self::Sftp(c) => c.read_link(path).await,
            Self::Ftp(c) => c.read_link(path).await,
        }
    }

    pub async fn rename(&self, from: &str, to: &str) -> Result<()> {
        match self {
            Self::Sftp(c) => c.rename(from, to).await,
            Self::Ftp(c) => c.rename(from, to).await,
        }
    }

    pub async fn chmod(&self, paths: &[String], mode: u32) -> Result<()> {
        match self {
            Self::Sftp(c) => c.chmod(paths, mode).await,
            Self::Ftp(c) => c.chmod(paths, mode).await,
        }
    }

    pub async fn create(&self, path: &str, kind: FileKind, link_target: Option<&str>) -> Result<()> {
        match self {
            Self::Sftp(c) => c.create(path, kind, link_target).await,
            Self::Ftp(c) => c.create(path, kind, link_target).await,
        }
    }

    pub async fn delete(&self, entries: &[PlannedDelete], cancel: Option<&CancelNotify<'_>>, handler: RespSftpDeleteProgressHandler) -> Result<()> {
        match self {
            Self::Sftp(c) => c.delete(entries, cancel, handler).await,
            Self::Ftp(c) => c.delete(entries, cancel, handler).await,
        }
    }

    pub async fn open_read(&self, path: &str) -> Result<Pin<Box<dyn AsyncRead + Send>>> {
        match self {
            Self::Sftp(c) => c.open_read(path).await,
            Self::Ftp(c) => c.open_read(path).await,
        }
    }

    pub async fn open_write(&self, path: &str, offset: Option<u64>, overwrite: bool, total: Option<u64>, handler: RespSftpTransferProgressHandler) -> Result<Pin<Box<dyn AsyncWrite + Send>>> {
        match self {
            Self::Sftp(c) => c.open_write(path, offset, overwrite, total, handler).await,
            Self::Ftp(c) => c.open_write(path, offset, overwrite, total, handler).await,
        }
    }
}

pub enum SessionClient {
    Local(Arc<ClientTransferLocal>),
    Remote(RemoteSessionClient),
}

impl SessionClient {
    pub async fn home_dir(&self) -> Result<String> {
        match self {
            Self::Local(c) => c.home_dir().await,
            Self::Remote(c) => c.home_dir().await,
        }
    }

    pub fn parent_path(&self, path: &str) -> Option<String> {
        match self {
            Self::Local(c) => c.parent_path(path),
            Self::Remote(c) => c.parent_path(path),
        }
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>> {
        match self {
            Self::Local(c) => c.list_dir(path).await,
            Self::Remote(c) => c.list_dir(path).await,
        }
    }

    pub async fn list_dir_resolving_links(&self, path: &str) -> Result<ResolvedListDir> {
        let requested_path = path.to_string();
        match self.list_dir(&requested_path).await {
            Ok(entries) => Ok(ResolvedListDir { effective_path: requested_path, entries }),
            Err(error) => {
                if let Ok(target) = self.read_link(&requested_path).await {
                    let resolved = path_util::resolve_link_target_path(&requested_path, &target, self.parent_path(&requested_path).as_deref());
                    let entries = self.list_dir(&resolved).await?;
                    return Ok(ResolvedListDir { effective_path: resolved, entries });
                }
                if let Ok(resolved) = self.realpath(&requested_path).await {
                    let entries = self.list_dir(&resolved).await?;
                    return Ok(ResolvedListDir { effective_path: resolved, entries });
                }
                Err(error)
            }
        }
    }

    pub async fn list_dir_for_transfer_plan(&self, path: &str, cancel: Option<&CancelNotify<'_>>) -> Result<Vec<FileEntry>> {
        match self {
            Self::Local(c) => c.list_dir_for_transfer_plan(path, cancel).await,
            Self::Remote(c) => c.list_dir_for_transfer_plan(path, cancel).await,
        }
    }

    pub async fn stat(&self, path: &str, kind: &FileKind) -> Result<FileEntry> {
        match self {
            Self::Local(c) => c.stat(path, kind).await,
            Self::Remote(c) => c.stat(path, kind).await,
        }
    }

    pub async fn stat_properties(&self, path: &str, kind: &FileKind) -> Result<FileStatProperties> {
        match self {
            Self::Local(c) => c.stat_properties(path, kind).await,
            Self::Remote(c) => c.stat_properties(path, kind).await,
        }
    }

    pub async fn exists(&self, path: &str) -> Result<bool> {
        match self {
            Self::Local(c) => c.exists(path).await,
            Self::Remote(c) => c.exists(path).await,
        }
    }

    pub async fn realpath(&self, path: &str) -> Result<String> {
        match self {
            Self::Local(c) => c.realpath(path).await,
            Self::Remote(c) => c.realpath(path).await,
        }
    }

    pub async fn read_link(&self, path: &str) -> Result<String> {
        match self {
            Self::Local(c) => c.read_link(path).await,
            Self::Remote(c) => c.read_link(path).await,
        }
    }

    pub async fn rename(&self, from: &str, to: &str) -> Result<()> {
        match self {
            Self::Local(c) => c.rename(from, to).await,
            Self::Remote(c) => c.rename(from, to).await,
        }
    }

    pub async fn chmod(&self, paths: &[String], mode: u32) -> Result<()> {
        match self {
            Self::Local(c) => c.chmod(paths, mode).await,
            Self::Remote(c) => c.chmod(paths, mode).await,
        }
    }

    pub async fn create(&self, path: &str, kind: FileKind, link_target: Option<&str>) -> Result<()> {
        match self {
            Self::Local(c) => c.create(path, kind, link_target).await,
            Self::Remote(c) => c.create(path, kind, link_target).await,
        }
    }

    pub async fn delete(&self, entries: &[PlannedDelete], cancel: Option<&CancelNotify<'_>>, handler: RespSftpDeleteProgressHandler) -> Result<()> {
        match self {
            Self::Local(c) => c.delete(entries, cancel, handler).await,
            Self::Remote(c) => c.delete(entries, cancel, handler).await,
        }
    }

    pub async fn open_read(&self, path: &str) -> Result<Pin<Box<dyn AsyncRead + Send>>> {
        match self {
            Self::Local(c) => c.open_read(path).await,
            Self::Remote(c) => c.open_read(path).await,
        }
    }

    pub async fn open_write(&self, path: &str, offset: Option<u64>, overwrite: bool, total: Option<u64>, handler: RespSftpTransferProgressHandler) -> Result<Pin<Box<dyn AsyncWrite + Send>>> {
        match self {
            Self::Local(c) => c.open_write(path, offset, overwrite, total, handler).await,
            Self::Remote(c) => c.open_write(path, offset, overwrite, total, handler).await,
        }
    }
}
