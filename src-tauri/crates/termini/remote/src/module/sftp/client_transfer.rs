use std::pin::Pin;
use anyhow::Result;
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite};
use crate::model::dto::cancel_session::CancelNotify;
use crate::module::common::client_terminal::EventHandler;
use crate::module::common::dto::file_stat_properties::FileStatProperties;
use crate::module::common::dto::file_entry::{FileEntry, FileKind, PlannedDelete};
use crate::model::resp::resp_sftp::{RespSftpDeleteProgressHandler, RespSftpTransferProgressHandler};

#[async_trait]
pub trait ClientTransfer: Send + Sync {
    type Event: Send + 'static;
    type Context: Send;

    fn get_id(&self) -> String;

    async fn close(&self) -> Result<()>;

    async fn home_dir(&self) -> Result<String>;

    async fn exists(&self, path: &str) -> Result<bool>;

    fn parent_path(&self, path: &str) -> Option<String>;

    async fn realpath(&self, path: &str) -> Result<String>;

    async fn read_link(&self, path: &str) -> Result<String>;

    async fn rename(&self, from: &str, to: &str) -> Result<()>;

    async fn chmod(&self, paths: &[String], mode: u32) -> Result<()>;

    async fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>>;

    async fn stat(&self, path: &str, kind: &FileKind) -> Result<FileEntry>;

    async fn open_read(&self, path: &str) -> Result<Pin<Box<dyn AsyncRead + Send>>>;

    async fn create(&self, path: &str, kind: FileKind, link_target: Option<&str>) -> Result<()>;

    async fn stat_properties(&self, path: &str, kind: &FileKind) -> Result<FileStatProperties>;

    async fn open(&self, context: &Self::Context, handler: EventHandler<Self::Event>) -> Result<()>;

    async fn list_dir_for_transfer_plan(&self, path: &str, cancel: Option<&CancelNotify<'_>>) -> Result<Vec<FileEntry>>;

    async fn delete(&self, entries: &[PlannedDelete], cancel: Option<&CancelNotify<'_>>, handler: RespSftpDeleteProgressHandler) -> Result<()>;

    async fn open_write(&self, path: &str, offset: Option<u64>, overwrite: bool, total: Option<u64>, handler: RespSftpTransferProgressHandler) -> Result<Pin<Box<dyn AsyncWrite + Send>>>;
}
