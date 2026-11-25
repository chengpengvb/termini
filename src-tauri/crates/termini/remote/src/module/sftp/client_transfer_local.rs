use std::pin::Pin;
use std::path::PathBuf;
use std::time::UNIX_EPOCH;
use crate::util::path_util;
#[cfg(target_os = "ios")]
use crate::util::workspace;
use async_trait::async_trait;
#[cfg(target_os = "ios")]
use crate::util::ios_local_dir;
use crate::util::id_factory::uuid;
use tokio::fs::{self, File, OpenOptions};
use anyhow::{anyhow, Context as _, Result};
use tokio::io::{AsyncRead, AsyncSeekExt, AsyncWrite};
use crate::module::common::client_terminal::EventHandler;
use crate::module::sftp::client_transfer::ClientTransfer;
use crate::model::resp::resp_sftp::RespSftpDeleteProgress;
use crate::module::common::dto::file_entry::PlannedDelete;
use crate::module::common::util::progress_writer::ProgressWriter;
use crate::module::common::dto::file_entry::{FileEntry, FileKind};
use crate::model::dto::cancel_session::{await_with_cancel, cancelled, CancelNotify};
use crate::module::common::dto::file_stat_properties::{DirStatProperties, FileStatProperties};
use crate::model::resp::resp_sftp::{RespSftpDeleteProgressHandler, RespSftpTransferProgressHandler};

pub struct ClientTransferLocal {
    id: String,
}

impl ClientTransferLocal {
    pub fn new() -> Self {
        Self { id: uuid() }
    }

    fn map_kind(ft: std::fs::FileType) -> FileKind {
        if ft.is_dir() {
            FileKind::Directory
        } else if ft.is_symlink() {
            FileKind::Symlink
        } else if ft.is_file() {
            FileKind::File
        } else {
            FileKind::Other
        }
    }

    #[cfg(unix)]
    fn file_permissions(meta: &std::fs::Metadata) -> Option<u32> {
        use std::os::unix::fs::PermissionsExt;
        Some(meta.permissions().mode())
    }

    #[cfg(not(unix))]
    fn file_permissions(_meta: &std::fs::Metadata) -> Option<u32> {
        None
    }

    #[cfg(windows)]
    fn resolve_is_hidden(_name: &str, meta: &std::fs::Metadata) -> bool {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x2 != 0
    }

    #[cfg(not(windows))]
    fn resolve_is_hidden(name: &str, _meta: &std::fs::Metadata) -> bool {
        name.starts_with('.')
    }

    #[cfg(unix)]
    fn resolve_uid_gid(meta: &std::fs::Metadata) -> (Option<u32>, Option<u32>) {
        use std::os::unix::fs::MetadataExt;
        (Some(meta.uid()), Some(meta.gid()))
    }

    #[cfg(not(unix))]
    fn resolve_uid_gid(_meta: &std::fs::Metadata) -> (Option<u32>, Option<u32>) {
        (None, None)
    }

    fn build_entry(name: String, path: String, meta: &std::fs::Metadata) -> FileEntry {
        let kind = Self::map_kind(meta.file_type());
        let size = if kind == FileKind::File { Some(meta.len()) } else { None };
        let mtime = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs());
        let hidden = Self::resolve_is_hidden(&name, meta);
        let (uid, gid) = Self::resolve_uid_gid(meta);
        FileEntry { hidden, name, path, kind, size, mtime, permissions: Self::file_permissions(meta), uid, gid, user: None, group: None }
    }

    async fn calc_dir_properties(root: &str) -> Result<DirStatProperties> {
        let mut total_size = 0u64;
        let mut file_count = 0u64;
        let mut dir_count = 0u64;
        let mut stack = vec![PathBuf::from(root)];
        while let Some(dir) = stack.pop() {
            let mut reader = fs::read_dir(&dir).await.with_context(|| format!("read_dir: {}", dir.display()))?;
            while let Some(entry) = reader.next_entry().await? {
                let ft = entry.file_type().await?;
                if ft.is_dir() {
                    dir_count += 1;
                    stack.push(entry.path());
                    continue;
                }
                file_count += 1;
                if ft.is_symlink() {
                    total_size += fs::symlink_metadata(entry.path()).await?.len();
                    continue;
                }
                total_size += entry.metadata().await?.len();
            }
        }
        Ok(DirStatProperties { total_size, file_count, dir_count })
    }

    fn ensure_scope(path: &str) -> Result<()> {
        #[cfg(target_os = "ios")]
        ios_local_dir::ensure_within_documents(path)?;
        #[cfg(not(target_os = "ios"))]
        let _ = path;
        Ok(())
    }

    #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
    fn drive_entry(name: String, path: String) -> FileEntry {
        FileEntry { hidden: false, name, path, kind: FileKind::Directory, size: None, mtime: None, permissions: None, uid: None, gid: None, user: None, group: None }
    }

    #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
    fn is_drive_list_path(path: &str) -> bool {
        #[cfg(windows)]
        {
            return path_util::is_computer_root(path);
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            return path_util::is_unix_volume_root(path);
        }
    }

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn volume_entry_name(path: &str) -> String {
        if path_util::is_computer_root(path) {
            return "/".to_string();
        }
        let trimmed = path.trim_end_matches('/');
        match trimmed.rsplit_once('/') {
            Some((_, name)) => name.to_string(),
            None => trimmed.to_string(),
        }
    }

    #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
    fn list_local_drives() -> Result<Vec<FileEntry>> {
        #[cfg(windows)]
        {
            #[link(name = "kernel32")]
            extern "system" {
                fn GetLogicalDrives() -> u32;
            }
            let mask = unsafe { GetLogicalDrives() };
            if mask == 0 {
                return Err(anyhow!("GetLogicalDrives failed"));
            }
            let mut entries = Vec::new();
            for index in 0..26u32 {
                if mask & (1 << index) == 0 {
                    continue;
                }
                let letter = (b'A' + index as u8) as char;
                let name = format!("{}:", letter);
                let path = format!("{}/", name);
                entries.push(Self::drive_entry(name, path));
            }
            return Ok(entries);
        }
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            let disks = sysinfo::Disks::new_with_refreshed_list();
            let mut entries = Vec::new();
            for disk in disks.list() {
                let raw = disk.mount_point().to_str().ok_or_else(|| anyhow!("non-UTF-8 mount point"))?;
                let path = path_util::normalize_frontend_path(raw);
                let name = Self::volume_entry_name(&path);
                entries.push(Self::drive_entry(name, path));
            }
            return Ok(entries);
        }
    }
}

#[async_trait]
impl ClientTransfer for ClientTransferLocal {
    type Event = ();
    type Context = ();

    fn get_id(&self) -> String {
        self.id.clone()
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }

    async fn home_dir(&self) -> Result<String> {
        #[cfg(windows)]
        {
            return std::env::var("USERPROFILE").map(|p| path_util::normalize_frontend_path(p.as_str())).map_err(|_| anyhow!("USERPROFILE is not set"));
        }
        #[cfg(target_os = "android")]
        {
            return Ok("/storage/emulated/0/".to_string());
        }
        #[cfg(target_os = "ios")]
        {
            let documents = ios_local_dir::documents_dir()?;
            fs::create_dir_all(&documents).await.with_context(|| format!("create documents dir: {}", documents.display()))?;
            let downloads = documents.join("Downloads");
            fs::create_dir_all(&downloads).await.with_context(|| format!("create downloads dir: {}", downloads.display()))?;
            let workspace = workspace::workspace_dir()?;
            fs::create_dir_all(&workspace).await.with_context(|| format!("create workspace dir: {}", workspace.display()))?;
            let deck = workspace::deck_dir()?;
            fs::create_dir_all(&deck).await.with_context(|| format!("create deck dir: {}", deck.display()))?;
            let raw = workspace.to_str().ok_or_else(|| anyhow!("non-UTF-8 workspace path"))?;
            return Ok(path_util::normalize_frontend_path(raw));
        }
        #[cfg(not(any(windows, target_os = "android", target_os = "ios")))]
        {
            return std::env::var("HOME").map(|p| path_util::normalize_frontend_path(p.as_str())).map_err(|_| anyhow!("HOME is not set"));
        }
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        Self::ensure_scope(path)?;
        fs::try_exists(path).await.with_context(|| format!("exists: {}", path))
    }

    fn parent_path(&self, path: &str) -> Option<String> {
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        {
            if path_util::is_unix_volume_root(path) {
                return None;
            }
            if path_util::is_computer_root(path) {
                return Some(path_util::unix_volume_root().to_string());
            }
        }
        let parent = path_util::parent_frontend_path(path)?;
        #[cfg(target_os = "ios")]
        {
            if !ios_local_dir::is_within_documents(&parent) {
                return None;
            }
        }
        Some(parent)
    }

    async fn realpath(&self, path: &str) -> Result<String> {
        Self::ensure_scope(path)?;
        let p = fs::canonicalize(path).await.with_context(|| format!("realpath: {}", path))?;
        let raw = p.to_str().map(|s| s.to_string()).ok_or_else(|| anyhow!("non-UTF-8 path"))?;
        let normalized = path_util::normalize_frontend_path(raw.as_str());
        Self::ensure_scope(&normalized)?;
        Ok(normalized)
    }

    async fn read_link(&self, path: &str) -> Result<String> {
        Self::ensure_scope(path)?;
        let target = fs::read_link(path).await.with_context(|| format!("read_link: {}", path))?;
        target.to_str().map(|s| path_util::normalize_frontend_path(s)).ok_or_else(|| anyhow!("non-UTF-8 symlink target"))
    }

    async fn rename(&self, from: &str, to: &str) -> Result<()> {
        Self::ensure_scope(from)?;
        Self::ensure_scope(to)?;
        let error = match fs::rename(from, to).await {
            Ok(_) => return Ok(()),
            Err(error) => error,
        };
        if !path_util::is_cross_device(&error) {
            return Err(anyhow::Error::new(error).context(format!("rename: {} -> {}", from, to)));
        }
        let meta = fs::metadata(from).await.with_context(|| format!("rename probe: {}", from))?;
        if meta.is_dir() {
            return Err(anyhow!("cross-device rename of directory not supported: {} -> {}", from, to));
        }
        fs::copy(from, to).await.with_context(|| format!("rename copy: {} -> {}", from, to))?;
        fs::remove_file(from).await.with_context(|| format!("rename cleanup: {}", from))?;
        Ok(())
    }

    async fn chmod(&self, paths: &[String], mode: u32) -> Result<()> {
        for path in paths {
            Self::ensure_scope(path)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for path in paths {
                let perms = std::fs::Permissions::from_mode(mode);
                fs::set_permissions(path, perms).await.with_context(|| format!("chmod: {}", path))?;
            }
        }
        #[cfg(not(unix))]
        let _ = (paths, mode);
        Ok(())
    }

    async fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>> {
        Self::ensure_scope(path)?;
        #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
        if Self::is_drive_list_path(path) {
            return Self::list_local_drives();
        }
        let owned_path = path.to_string();
        tokio::task::spawn_blocking(move || -> Result<Vec<FileEntry>> {
            let mut entries = Vec::new();
            for entry in std::fs::read_dir(&owned_path).with_context(|| format!("list_dir: {}", owned_path))? {
                let entry = entry.with_context(|| format!("list_dir entry: {}", owned_path))?;
                let name = entry.file_name().to_string_lossy().into_owned();
                let full_path = path_util::normalize_pathbuf_lossy(entry.path().as_path());
                entries.push(Self::build_entry(name, full_path, &entry.metadata().with_context(|| format!("list_dir metadata: {}", owned_path))?));
            }
            Ok(entries)
        })
        .await
        .map_err(|error| anyhow!("list_dir task join error: {}: {}", path, error))?
    }

    async fn stat(&self, path: &str, kind: &FileKind) -> Result<FileEntry> {
        Self::ensure_scope(path)?;
        let p = PathBuf::from(path);
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.to_string());
        let meta =
            if *kind == FileKind::Symlink { fs::symlink_metadata(&p).await.with_context(|| format!("lstat: {}", path))? } else { fs::metadata(&p).await.with_context(|| format!("stat: {}", path))? };
        let mtime = meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs());
        let size = if *kind == FileKind::Directory { None } else { Some(meta.len()) };
        let hidden = Self::resolve_is_hidden(&name, &meta);
        let (uid, gid) = Self::resolve_uid_gid(&meta);
        Ok(FileEntry { hidden, name, path: path_util::normalize_frontend_path(path), kind: *kind, size, mtime, permissions: Self::file_permissions(&meta), uid, gid, user: None, group: None })
    }

    async fn open_read(&self, path: &str) -> Result<Pin<Box<dyn AsyncRead + Send>>> {
        Self::ensure_scope(path)?;
        let file = File::open(path).await.with_context(|| format!("open_read: {}", path))?;
        Ok(Box::pin(file))
    }

    async fn create(&self, path: &str, kind: FileKind, link_target: Option<&str>) -> Result<()> {
        Self::ensure_scope(path)?;
        match kind {
            FileKind::Directory => fs::create_dir_all(path).await.with_context(|| format!("create_dir: {}", path)),
            FileKind::File => {
                if let Some(parent) = PathBuf::from(path).parent() {
                    fs::create_dir_all(parent).await.with_context(|| format!("create parent: {}", path))?;
                }
                File::create(path).await.with_context(|| format!("create_file: {}", path)).map(|_| ())
            }
            FileKind::Symlink => {
                let target = link_target.ok_or_else(|| anyhow!("symlink requires link_target"))?;
                #[cfg(unix)]
                fs::symlink(target, path).await.with_context(|| format!("symlink: {} -> {}", path, target))?;
                #[cfg(windows)]
                {
                    let is_dir = fs::metadata(target).await.map(|m| m.is_dir()).unwrap_or(false);
                    if is_dir {
                        fs::symlink_dir(target, path).await.with_context(|| format!("symlink_dir: {}", path))?;
                    } else {
                        fs::symlink_file(target, path).await.with_context(|| format!("symlink_file: {}", path))?;
                    }
                }
                Ok(())
            }
            FileKind::Other => Err(anyhow!("cannot create entry of kind Other")),
            FileKind::Parent => Err(anyhow!("cannot create entry of kind Parent")),
        }
    }

    async fn stat_properties(&self, path: &str, kind: &FileKind) -> Result<FileStatProperties> {
        let entry = self.stat(path, kind).await?;
        let dir = if *kind == FileKind::Directory { Some(Self::calc_dir_properties(path).await?) } else { None };
        Ok(FileStatProperties { path: path_util::normalize_frontend_path(path), entry, dir })
    }

    async fn open(&self, _context: &Self::Context, _handler: EventHandler<Self::Event>) -> Result<()> {
        Ok(())
    }

    async fn list_dir_for_transfer_plan(&self, path: &str, cancel: Option<&CancelNotify<'_>>) -> Result<Vec<FileEntry>> {
        Self::ensure_scope(path)?;
        #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
        if Self::is_drive_list_path(path) {
            return Self::list_local_drives();
        }
        let owned_path = path.to_string();
        let list_entries = || async {
            tokio::task::spawn_blocking(move || -> Result<Vec<FileEntry>> {
                let mut entries = Vec::new();
                for entry in std::fs::read_dir(&owned_path).with_context(|| format!("list_dir_for_transfer_plan: {}", owned_path))? {
                    let entry = entry.with_context(|| format!("list_dir_for_transfer_plan entry: {}", owned_path))?;
                    let name = entry.file_name().to_string_lossy().into_owned();
                    let full_path = path_util::normalize_pathbuf_lossy(entry.path().as_path());
                    #[cfg(windows)]
                    let (kind, hidden, size, permissions) = {
                        let meta = entry.metadata().with_context(|| format!("list_dir_for_transfer_plan metadata: {}", owned_path))?;
                        let kind = Self::map_kind(meta.file_type());
                        let hidden = Self::resolve_is_hidden(&name, &meta);
                        let size = if kind == FileKind::Directory { None } else { Some(meta.len()) };
                        (kind, hidden, size, Self::file_permissions(&meta))
                    };
                    #[cfg(not(windows))]
                    let (kind, hidden, size, permissions) = {
                        let meta = entry.metadata().with_context(|| format!("list_dir_for_transfer_plan metadata: {}", owned_path))?;
                        let kind = Self::map_kind(meta.file_type());
                        let hidden = name.starts_with('.');
                        let size = if kind == FileKind::Directory { None } else { Some(meta.len()) };
                        (kind, hidden, size, Self::file_permissions(&meta))
                    };
                    entries.push(FileEntry { hidden, name, path: full_path, kind, size, mtime: None, permissions, uid: None, gid: None, user: None, group: None });
                }
                Ok(entries)
            })
            .await
            .map_err(|error| anyhow!("list_dir_for_transfer_plan task join error: {}: {}", path, error))?
        };
        match cancel {
            None => list_entries().await,
            Some(notify) => await_with_cancel(notify, list_entries).await,
        }
    }

    async fn delete(&self, entries: &[PlannedDelete], cancel: Option<&CancelNotify<'_>>, handler: RespSftpDeleteProgressHandler) -> Result<()> {
        let mut files_done: u32 = 0;
        let files_total = entries.len() as u32;
        for entry in entries {
            Self::ensure_scope(&entry.path)?;
            if let Some(notify) = cancel {
                if notify.is_cancelled() {
                    return Err(cancelled());
                }
            }
            if entry.kind == FileKind::Directory {
                fs::remove_dir(&entry.path).await.with_context(|| format!("delete dir: {}", entry.path))?;
            } else {
                fs::remove_file(&entry.path).await.with_context(|| format!("delete file: {}", entry.path))?;
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
        Self::ensure_scope(path)?;
        if let Some(parent) = PathBuf::from(path).parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent).await.with_context(|| format!("create parent: {}", path))?;
            }
        }
        let file = match offset {
            Some(pos) => {
                let mut f = OpenOptions::new().write(true).create(true).open(path).await.with_context(|| format!("open_write resume: {}", path))?;
                f.seek(std::io::SeekFrom::Start(pos)).await.with_context(|| format!("seek: {}", path))?;
                f
            }
            None => {
                if overwrite {
                    OpenOptions::new().write(true).create(true).truncate(true).open(path).await.with_context(|| format!("open_write overwrite: {}", path))?
                } else {
                    OpenOptions::new().write(true).create_new(true).open(path).await.with_context(|| format!("open_write exclusive: {}", path))?
                }
            }
        };
        Ok(Box::pin(ProgressWriter::new(Box::pin(file), path.to_string(), total, offset.unwrap_or(0), handler)))
    }
}
