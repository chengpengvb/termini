use std::pin::Pin;
use std::sync::Arc;
use crate::util::path_util;
use async_trait::async_trait;
use crate::util::id_factory::uuid;
use russh_sftp::client::SftpSession;
use russh_sftp::client::fs::Metadata;
use tokio::sync::Mutex as AsyncMutex;
use anyhow::{anyhow, Context as _, Result};
use summer::plugin::component::ComponentRef;
use crate::{OpenSshStep, RespSshOpenStepType};
use russh_sftp::protocol::{FileType, OpenFlags};
use tokio::io::{AsyncRead, AsyncSeekExt, AsyncWrite};
use crate::module::common::dto::ssh_session::SshSession;
use crate::module::common::client_terminal::EventHandler;
use crate::module::sftp::client_transfer::ClientTransfer;
use crate::model::resp::resp_sftp::RespSftpDeleteProgress;
use crate::module::common::dto::file_entry::PlannedDelete;
use crate::module::common::context::context_ssh::ContextSsh;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::module::common::dto::ssh_channel_kind::SshChannelKind;
use crate::module::common::util::progress_writer::ProgressWriter;
use crate::module::common::dto::file_entry::{FileEntry, FileKind};
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::model::dto::cancel_session::{await_with_cancel, cancelled, CancelNotify};
use crate::module::common::dto::file_stat_properties::{DirStatProperties, FileStatProperties};
use crate::model::resp::resp_sftp::{RespSftpDeleteProgressHandler, RespSftpTransferProgressHandler};

struct SftpState {
    channel_id: String,
    session: Arc<SftpSession>,
}

#[derive(Clone)]
pub struct ClientTransferSftp {
    session_id: String,
    client_transfer_id: String,
    factory: ComponentRef<ClientSessionFactory>,
    state: Arc<AsyncMutex<Option<SftpState>>>,
}

impl ClientTransferSftp {
    pub fn new(session_id: String, factory: ComponentRef<ClientSessionFactory>) -> Self {
        Self { session_id, client_transfer_id: uuid(), factory, state: Arc::new(AsyncMutex::new(None)) }
    }

    async fn session(&self) -> Result<Arc<SftpSession>> {
        let guard = self.state.lock().await;
        guard.as_ref().map(|s| s.session.clone()).ok_or_else(|| anyhow!("sftp not opened"))
    }

    fn map_kind(file_type: FileType) -> FileKind {
        match file_type {
            FileType::File => FileKind::File,
            FileType::Other => FileKind::Other,
            FileType::Dir => FileKind::Directory,
            FileType::Symlink => FileKind::Symlink,
        }
    }

    fn build_entry(name: String, path: String, meta: &Metadata) -> FileEntry {
        let kind = Self::map_kind(meta.file_type());
        let size = if kind == FileKind::Directory { None } else { meta.size };
        let hidden = name.starts_with('.');
        FileEntry {
            hidden,
            name,
            path,
            kind,
            size,
            uid: meta.uid,
            gid: meta.gid,
            user: meta.user.clone(),
            group: meta.group.clone(),
            permissions: meta.permissions,
            mtime: meta.mtime.map(|t| t as u64),
        }
    }

    async fn calc_dir_properties(session: &SftpSession, root: &str) -> Result<DirStatProperties> {
        let mut total_size = 0u64;
        let mut file_count = 0u64;
        let mut dir_count = 0u64;
        let mut stack = vec![root.to_string()];
        while let Some(dir) = stack.pop() {
            let read_dir = session.read_dir(dir.as_str()).await.with_context(|| format!("sftp stat_properties read_dir: {}", dir))?;
            for entry in read_dir {
                let child = Self::posix_join(&dir, &entry.file_name());
                let meta = entry.metadata();
                if entry.file_type() == FileType::Dir {
                    dir_count += 1;
                    stack.push(child);
                    continue;
                }
                if let Some(size) = meta.size {
                    total_size += size;
                }
                file_count += 1;
            }
        }
        Ok(DirStatProperties { total_size, file_count, dir_count })
    }

    fn posix_name(path: &str) -> String {
        let trimmed = path.trim_end_matches('/');
        match trimmed.rsplit('/').next() {
            Some(name) if !name.is_empty() => name.to_string(),
            _ => path.to_string(),
        }
    }

    fn posix_join(base: &str, name: &str) -> String {
        let base = base.trim_end_matches('/');
        if base.is_empty() {
            return format!("/{}", name);
        }
        format!("{}/{}", base, name)
    }

    async fn create_dir_all(session: &SftpSession, path: &str) -> Result<()> {
        let trimmed = path.trim_end_matches('/');
        if trimmed.is_empty() || trimmed == "/" {
            return Ok(());
        }
        let mut current = String::new();
        for part in trimmed.split('/') {
            if part.is_empty() {
                continue;
            }
            current.push('/');
            current.push_str(part);
            if session.try_exists(current.as_str()).await.with_context(|| format!("sftp try_exists: {}", current))? {
                continue;
            }
            session.create_dir(current.as_str()).await.with_context(|| format!("sftp create_dir: {}", current))?;
        }
        Ok(())
    }
}

#[async_trait]
impl ClientTransfer for ClientTransferSftp {
    type Event = RespEventSftp;
    type Context = ContextSsh;

    fn get_id(&self) -> String {
        self.client_transfer_id.clone()
    }

    async fn close(&self) -> Result<()> {
        let state = {
            let mut guard = self.state.lock().await;
            guard.take()
        };
        if let Some(state) = state {
            let _ = state.session.close().await;
            self.factory.close(&state.channel_id).await?;
        }
        Ok(())
    }

    async fn home_dir(&self) -> Result<String> {
        let session = self.session().await?;
        let raw = session.canonicalize(".").await.with_context(|| "sftp resolve home directory")?;
        Ok(path_util::normalize_to_slash(raw.as_str()))
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let session = self.session().await?;
        session.try_exists(path).await.with_context(|| format!("sftp exists: {}", path))
    }

    fn parent_path(&self, path: &str) -> Option<String> {
        path_util::parent_frontend_path(path)
    }

    async fn realpath(&self, path: &str) -> Result<String> {
        let session = self.session().await?;
        let raw = session.canonicalize(path).await.with_context(|| format!("sftp realpath: {}", path))?;
        Ok(path_util::normalize_to_slash(raw.as_str()))
    }

    async fn read_link(&self, path: &str) -> Result<String> {
        let session = self.session().await?;
        let raw = session.read_link(path).await.with_context(|| format!("sftp read_link: {}", path))?;
        Ok(path_util::normalize_to_slash(raw.as_str()))
    }

    async fn rename(&self, from: &str, to: &str) -> Result<()> {
        let session = self.session().await?;
        session.rename(from, to).await.with_context(|| format!("sftp rename: {} -> {}", from, to))
    }

    async fn chmod(&self, paths: &[String], mode: u32) -> Result<()> {
        let session = self.session().await?;
        let attrs = Metadata { size: None, uid: None, user: None, gid: None, group: None, permissions: Some(mode), atime: None, mtime: None };
        for path in paths {
            session.set_metadata(path, attrs.clone()).await.with_context(|| format!("sftp chmod: {}", path))?;
        }
        Ok(())
    }

    async fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>> {
        let session = self.session().await?;
        let read_dir = session.read_dir(path).await.with_context(|| format!("sftp list_dir: {}", path))?;
        let entries: Vec<FileEntry> = read_dir
            .map(|entry| {
                let name = entry.file_name();
                let full_path = Self::posix_join(path, &name);
                Self::build_entry(name, full_path, &entry.metadata())
            })
            .collect();
        Ok(entries)
    }

    async fn stat(&self, path: &str, kind: &FileKind) -> Result<FileEntry> {
        let session = self.session().await?;
        let meta = if *kind == FileKind::Symlink {
            session.symlink_metadata(path).await.with_context(|| format!("sftp lstat: {}", path))?
        } else {
            session.metadata(path).await.with_context(|| format!("sftp stat: {}", path))?
        };
        Ok(Self::build_entry(Self::posix_name(path), path_util::normalize_to_slash(path), &meta))
    }

    async fn open_read(&self, path: &str) -> Result<Pin<Box<dyn AsyncRead + Send>>> {
        let session = self.session().await?;
        let file = session.open(path).await.with_context(|| format!("sftp open_read: {}", path))?;
        Ok(Box::pin(file))
    }

    async fn create(&self, path: &str, kind: FileKind, link_target: Option<&str>) -> Result<()> {
        let session = self.session().await?;
        match kind {
            FileKind::Directory => Self::create_dir_all(&session, path).await,
            FileKind::File => {
                if let Some(parent) = self.parent_path(path) {
                    Self::create_dir_all(&session, &parent).await?;
                }
                session.create(path).await.with_context(|| format!("sftp create_file: {}", path)).map(|_| ())
            }
            FileKind::Symlink => {
                let target = link_target.ok_or_else(|| anyhow!("symlink requires link_target"))?;
                session.symlink(path, target).await.with_context(|| format!("sftp symlink: {} -> {}", path, target))
            }
            FileKind::Other => Err(anyhow!("cannot create entry of kind Other")),
            FileKind::Parent => Err(anyhow!("cannot create entry of kind Parent")),
        }
    }

    async fn stat_properties(&self, path: &str, kind: &FileKind) -> Result<FileStatProperties> {
        let entry = self.stat(path, kind).await?;
        if *kind != FileKind::Directory {
            return Ok(FileStatProperties { path: path_util::normalize_to_slash(path), entry, dir: None });
        }
        let session = self.session().await?;
        let dir = Self::calc_dir_properties(&session, path).await?;
        Ok(FileStatProperties { path: path_util::normalize_to_slash(path), entry, dir: Some(dir) })
    }

    async fn open(&self, context: &Self::Context, handler: EventHandler<Self::Event>) -> Result<()> {
        let SshSession { channel_id, channel } = self.factory.connect(self.session_id.clone(), context, None, handler.clone(), SshChannelKind::Sftp, None).await?;
        let _ = handler(RespEventSftp::OnOpenStep(crate::model::resp::resp_sftp::RespSftpOpenStep {
            client_transfer_id: self.client_transfer_id.clone(),
            step: OpenSshStep { progress: 100.0, step_type: RespSshOpenStepType::SessionOpened, data: None },
        }))
        .await;

        let session = SftpSession::new(channel.into_stream()).await.with_context(|| "sftp session init failed")?;
        let mut guard = self.state.lock().await;
        *guard = Some(SftpState { channel_id, session: Arc::new(session) });
        Ok(())
    }

    async fn list_dir_for_transfer_plan(&self, path: &str, cancel: Option<&CancelNotify<'_>>) -> Result<Vec<FileEntry>> {
        let list_entries = || async {
            let session = self.session().await?;
            let read_dir = session.read_dir(path).await.with_context(|| format!("sftp list_dir_for_transfer_plan: {}", path))?;
            let entries: Vec<FileEntry> = read_dir
                .map(|entry| {
                    let name = entry.file_name();
                    let full_path = Self::posix_join(path, &name);
                    let meta = entry.metadata();
                    let kind = Self::map_kind(meta.file_type());
                    let size = if kind == FileKind::Directory { None } else { meta.size };
                    let hidden = name.starts_with('.');
                    FileEntry { hidden, name, path: full_path, kind, size, mtime: None, permissions: meta.permissions, uid: None, gid: None, user: None, group: None }
                })
                .collect();
            Ok(entries)
        };
        match cancel {
            Some(notify) => await_with_cancel(notify, list_entries).await,
            None => list_entries().await,
        }
    }

    async fn delete(&self, entries: &[PlannedDelete], cancel: Option<&CancelNotify<'_>>, handler: RespSftpDeleteProgressHandler) -> Result<()> {
        let session = self.session().await?;
        let files_total = entries.len() as u32;
        let mut files_done: u32 = 0;
        for entry in entries {
            if let Some(notify) = cancel {
                if notify.is_cancelled() {
                    return Err(cancelled());
                }
            }
            if entry.kind == FileKind::Directory {
                session.remove_dir(entry.path.as_str()).await.with_context(|| format!("sftp delete dir: {}", entry.path))?;
            } else {
                session.remove_file(entry.path.as_str()).await.with_context(|| format!("sftp delete file: {}", entry.path))?;
            }
            files_done += 1;
            let percent = if files_total > 0 { (files_done as f32 / files_total as f32 * 100.0).min(100.0) } else { 100.0 };
            let finish = files_done == files_total;
            let progress = RespSftpDeleteProgress { file: entry.path.clone(), finish, percent, files_done, files_total, resp_id: String::new(), client_transfer_id: String::new() };
            let _ = handler(progress).await;
        }
        Ok(())
    }

    async fn open_write(&self, path: &str, offset: Option<u64>, overwrite: bool, total: Option<u64>, handler: RespSftpTransferProgressHandler) -> Result<Pin<Box<dyn AsyncWrite + Send>>> {
        let session = self.session().await?;
        if let Some(parent) = self.parent_path(path) {
            Self::create_dir_all(&session, &parent).await?;
        }
        let flags = match offset {
            Some(_) => OpenFlags::CREATE | OpenFlags::WRITE,
            None if overwrite => OpenFlags::CREATE | OpenFlags::WRITE | OpenFlags::TRUNCATE,
            None => OpenFlags::CREATE | OpenFlags::WRITE | OpenFlags::EXCLUDE,
        };
        let mut file = session.open_with_flags(path, flags).await.with_context(|| format!("sftp open_write: {}", path))?;
        if let Some(pos) = offset {
            file.seek(std::io::SeekFrom::Start(pos)).await.with_context(|| format!("sftp seek: {}", path))?;
        }
        Ok(Box::pin(ProgressWriter::new(Box::pin(file), path.to_string(), total, offset.unwrap_or(0), handler)))
    }
}
