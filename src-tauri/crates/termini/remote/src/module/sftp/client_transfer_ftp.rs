use std::pin::Pin;
use std::sync::Arc;
use suppaftp::Status;
use std::str::FromStr;
use std::time::Duration;
use rustls::ClientConfig;
use summer::tracing::warn;
use crate::util::path_util;
use async_trait::async_trait;
use suppaftp::types::FileType;
use crate::util::id_factory::uuid;
use tokio::sync::Mutex as AsyncMutex;
use tokio::io::{AsyncRead, AsyncWrite};
use suppaftp::list::File as FtpListFile;
use anyhow::{anyhow, Context as _, Result};
use crate::{OpenSshStep, RespSshOpenStepType};
use tokio_rustls::TlsConnector as RustlsTlsConnector;
use crate::module::common::client_terminal::EventHandler;
use crate::module::sftp::client_transfer::ClientTransfer;
use crate::module::common::context::context_ftp::ContextFtp;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use crate::module::common::util::progress_writer::ProgressWriter;
use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};
use crate::module::common::dto::file_entry::{FileEntry, FileKind, PlannedDelete};
use suppaftp::tokio::{AsyncFtpStream, AsyncRustlsConnector, AsyncRustlsFtpStream};
use crate::model::dto::cancel_session::{await_with_cancel, cancelled, CancelNotify};
use crate::module::common::dto::file_stat_properties::{DirStatProperties, FileStatProperties};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use crate::model::config_ftp::{FTP_SECURITY_EXPLICIT, FTP_SECURITY_IMPLICIT, FTP_SECURITY_NONE};
use crate::model::resp::resp_sftp::{RespSftpDeleteProgress, RespSftpDeleteProgressHandler, RespSftpOpenStep, RespSftpTransferProgressHandler};

const FTP_CONTROL_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(Debug)]
struct NoServerCertVerify {
    algs: rustls::crypto::WebPkiSupportedAlgorithms,
}

impl NoServerCertVerify {
    fn new() -> Self {
        Self { algs: rustls::crypto::ring::default_provider().signature_verification_algorithms }
    }
}

impl ServerCertVerifier for NoServerCertVerify {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        server_name: &ServerName<'_>,
        ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, RustlsError> {
        let _ = (end_entity, intermediates, server_name, ocsp_response, now);
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, RustlsError> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.algs)
    }

    fn verify_tls13_signature(&self, message: &[u8], cert: &CertificateDer<'_>, dss: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, RustlsError> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.algs)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.algs.supported_schemes()
    }
}

enum FtpClient {
    Plain(AsyncFtpStream),
    Tls(AsyncRustlsFtpStream),
}

#[derive(Clone)]
pub struct ClientTransferFtp {
    client_transfer_id: String,
    state: Arc<AsyncMutex<Option<FtpClient>>>,
}

impl ClientTransferFtp {
    pub fn new() -> Self {
        Self { client_transfer_id: uuid(), state: Arc::new(AsyncMutex::new(None)) }
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

    fn build_entry(name: String, path: String, file: &FtpListFile) -> FileEntry {
        let kind = if file.is_directory() { FileKind::Directory } else { FileKind::File };
        let hidden = name.starts_with('.');
        FileEntry {
            hidden,
            name,
            path,
            kind,
            size: if kind == FileKind::Directory { None } else { Some(file.size() as u64) },
            uid: None,
            gid: None,
            user: None,
            group: None,
            permissions: None,
            mtime: file.modified().duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_secs()),
        }
    }

    fn parse_entries(path: &str, lines: Vec<String>) -> Result<Vec<FileEntry>> {
        let mut entries = Vec::new();
        for line in lines {
            let file = FtpListFile::from_str(line.as_str()).map_err(|error| anyhow!("ftp list parse failed: {} line={}", error, line))?;
            let name = file.name().to_string();
            if name == "." || name == ".." {
                continue;
            }
            entries.push(Self::build_entry(name.clone(), Self::posix_join(path, &name), &file));
        }
        Ok(entries)
    }

    fn build_tls_config(ignore_cert: bool) -> Result<ClientConfig> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        if ignore_cert {
            return Ok(ClientConfig::builder().dangerous().with_custom_certificate_verifier(Arc::new(NoServerCertVerify::new())).with_no_client_auth());
        }
        let roots = rustls::RootCertStore { roots: webpki_roots::TLS_SERVER_ROOTS.to_vec() };
        Ok(ClientConfig::builder().with_root_certificates(roots).with_no_client_auth())
    }

    async fn connect_client(host: &str, port: u16, timeout: Option<Duration>, security: i16, ignore_cert: bool) -> Result<FtpClient> {
        let addr = format!("{}:{}", host, port);
        if security == FTP_SECURITY_NONE {
            let stream = Self::connect_plain(addr.as_str(), timeout).await?;
            return Ok(FtpClient::Plain(stream));
        }
        let connector = AsyncRustlsConnector::from(RustlsTlsConnector::from(Arc::new(Self::build_tls_config(ignore_cert)?)));
        if security == FTP_SECURITY_IMPLICIT {
            let stream = if let Some(duration) = timeout {
                tokio::time::timeout(duration, AsyncRustlsFtpStream::connect_secure_implicit(addr.as_str(), connector, host)).await.map_err(|_| anyhow!("ftps implicit connect timeout"))?
            } else {
                AsyncRustlsFtpStream::connect_secure_implicit(addr.as_str(), connector, host).await
            }
            .map_err(|error| anyhow!("ftps implicit connect failed: {}", error))?;
            let mut stream = stream;
            stream.custom_command("PBSZ 0", &[Status::CommandOk]).await.map_err(|error| anyhow!("ftps implicit pbsz failed: {}", error))?;
            stream.custom_command("PROT P", &[Status::CommandOk]).await.map_err(|error| anyhow!("ftps implicit prot failed: {}", error))?;
            return Ok(FtpClient::Tls(stream));
        }
        if security != FTP_SECURITY_EXPLICIT {
            return Err(anyhow!("ftp security invalid: {}", security));
        }
        let stream = Self::connect_tls_plain(addr.as_str(), timeout).await?;
        let stream = stream.into_secure(connector, host).await.map_err(|error| anyhow!("ftps explicit upgrade failed: {}", error))?;
        Ok(FtpClient::Tls(stream))
    }

    async fn connect_plain(addr: &str, timeout: Option<Duration>) -> Result<AsyncFtpStream> {
        let stream = if let Some(duration) = timeout {
            let sock = tokio::net::lookup_host(addr).await.with_context(|| format!("ftp resolve {}", addr))?.next().ok_or_else(|| anyhow!("ftp resolve empty: {}", addr))?;
            AsyncFtpStream::connect_timeout(sock, duration).await
        } else {
            AsyncFtpStream::connect(addr).await
        };
        stream.map_err(|error| anyhow!("ftp connect failed: {}", error))
    }

    async fn connect_tls_plain(addr: &str, timeout: Option<Duration>) -> Result<AsyncRustlsFtpStream> {
        let stream = if let Some(duration) = timeout {
            let sock = tokio::net::lookup_host(addr).await.with_context(|| format!("ftps resolve {}", addr))?.next().ok_or_else(|| anyhow!("ftps resolve empty: {}", addr))?;
            AsyncRustlsFtpStream::connect_timeout(sock, duration).await
        } else {
            AsyncRustlsFtpStream::connect(addr).await
        };
        stream.map_err(|error| anyhow!("ftps connect failed: {}", error))
    }

    async fn list_entries(client: &mut FtpClient, path: &str) -> Result<Vec<FileEntry>> {
        let lines = client.list(Some(path)).await?;
        Self::parse_entries(path, lines)
    }

    async fn create_dir_all(client: &mut FtpClient, path: &str) -> Result<()> {
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
            if Self::entry_exists(client, current.as_str()).await? {
                continue;
            }
            client.mkdir(current.as_str()).await?;
        }
        Ok(())
    }

    async fn entry_exists(client: &mut FtpClient, path: &str) -> Result<bool> {
        if path == "/" || path.is_empty() {
            return Ok(true);
        }
        let Some(parent) = Self::parent_of(path) else {
            return Ok(true);
        };
        let name = Self::posix_name(path);
        let entries = Self::list_entries(client, parent.as_str()).await?;
        Ok(entries.iter().any(|entry| entry.name == name))
    }

    fn parent_of(path: &str) -> Option<String> {
        path_util::parent_frontend_path(path)
    }

    async fn calc_dir_properties(client: &mut FtpClient, root: &str) -> Result<DirStatProperties> {
        let mut total_size = 0u64;
        let mut file_count = 0u64;
        let mut dir_count = 0u64;
        let mut stack = vec![root.to_string()];
        while let Some(dir) = stack.pop() {
            let entries = Self::list_entries(client, dir.as_str()).await?;
            for entry in entries {
                if entry.kind == FileKind::Directory {
                    dir_count += 1;
                    stack.push(entry.path);
                    continue;
                }
                if let Some(size) = entry.size {
                    total_size += size;
                }
                file_count += 1;
            }
        }
        Ok(DirStatProperties { total_size, file_count, dir_count })
    }
}

impl FtpClient {
    async fn login(&mut self, user: &str, password: &str) -> Result<()> {
        tokio::time::timeout(FTP_CONTROL_TIMEOUT, async {
            match self {
                Self::Plain(stream) => stream.login(user, password).await,
                Self::Tls(stream) => stream.login(user, password).await,
            }
            .map_err(|error| anyhow!("ftp login failed: {}", error))
        })
        .await
        .map_err(|error| anyhow!("ftp login timed out: {}", error))?
    }

    async fn set_binary(&mut self) -> Result<()> {
        tokio::time::timeout(FTP_CONTROL_TIMEOUT, async {
            match self {
                Self::Plain(stream) => stream.transfer_type(FileType::Binary).await,
                Self::Tls(stream) => stream.transfer_type(FileType::Binary).await,
            }
            .map_err(|error| anyhow!("ftp type binary failed: {}", error))
        })
        .await
        .map_err(|error| anyhow!("ftp set_binary timed out: {}", error))?
    }

    async fn pwd(&mut self) -> Result<String> {
        tokio::time::timeout(FTP_CONTROL_TIMEOUT, async {
            match self {
                Self::Plain(stream) => stream.pwd().await,
                Self::Tls(stream) => stream.pwd().await,
            }
            .map_err(|error| anyhow!("ftp pwd failed: {}", error))
        })
        .await
        .map_err(|error| anyhow!("ftp pwd timed out: {}", error))?
    }

    async fn cwd(&mut self, path: &str) -> Result<()> {
        tokio::time::timeout(FTP_CONTROL_TIMEOUT, async {
            match self {
                Self::Plain(stream) => stream.cwd(path).await,
                Self::Tls(stream) => stream.cwd(path).await,
            }
            .map_err(|error| anyhow!("ftp cwd failed: {}", error))
        })
        .await
        .map_err(|error| anyhow!("ftp cwd timed out: {}", error))?
    }

    async fn list(&mut self, path: Option<&str>) -> Result<Vec<String>> {
        tokio::time::timeout(Duration::from_secs(15), async {
            match self {
                Self::Plain(stream) => stream.list(path).await,
                Self::Tls(stream) => stream.list(path).await,
            }
            .map_err(|error| anyhow!("ftp list failed: {}", error))
        })
        .await
        .map_err(|error| anyhow!("ftp list timed out: {}", error))?
    }

    async fn rename(&mut self, from: &str, to: &str) -> Result<()> {
        tokio::time::timeout(FTP_CONTROL_TIMEOUT, async {
            match self {
                Self::Plain(stream) => stream.rename(from, to).await,
                Self::Tls(stream) => stream.rename(from, to).await,
            }
            .map_err(|error| anyhow!("ftp rename failed: {}", error))
        })
        .await
        .map_err(|error| anyhow!("ftp rename timed out: {}", error))?
    }

    async fn mkdir(&mut self, path: &str) -> Result<()> {
        tokio::time::timeout(FTP_CONTROL_TIMEOUT, async {
            match self {
                Self::Plain(stream) => stream.mkdir(path).await,
                Self::Tls(stream) => stream.mkdir(path).await,
            }
            .map_err(|error| anyhow!("ftp mkdir failed: {}", error))
        })
        .await
        .map_err(|error| anyhow!("ftp mkdir timed out: {}", error))?
    }

    async fn rm(&mut self, path: &str) -> Result<()> {
        tokio::time::timeout(FTP_CONTROL_TIMEOUT, async {
            match self {
                Self::Plain(stream) => stream.rm(path).await,
                Self::Tls(stream) => stream.rm(path).await,
            }
            .map_err(|error| anyhow!("ftp rm failed: {}", error))
        })
        .await
        .map_err(|error| anyhow!("ftp rm timed out: {}", error))?
    }

    async fn rmdir(&mut self, path: &str) -> Result<()> {
        tokio::time::timeout(FTP_CONTROL_TIMEOUT, async {
            match self {
                Self::Plain(stream) => stream.rmdir(path).await,
                Self::Tls(stream) => stream.rmdir(path).await,
            }
            .map_err(|error| anyhow!("ftp rmdir failed: {}", error))
        })
        .await
        .map_err(|error| anyhow!("ftp rmdir timed out: {}", error))?
    }

    async fn resume(&mut self, offset: u64) -> Result<()> {
        tokio::time::timeout(FTP_CONTROL_TIMEOUT, async {
            match self {
                Self::Plain(stream) => stream.resume_transfer(offset as usize).await,
                Self::Tls(stream) => stream.resume_transfer(offset as usize).await,
            }
            .map_err(|error| anyhow!("ftp rest failed: {}", error))
        })
        .await
        .map_err(|error| anyhow!("ftp resume timed out: {}", error))?
    }

    async fn quit(&mut self) -> Result<()> {
        tokio::time::timeout(FTP_CONTROL_TIMEOUT, async {
            match self {
                Self::Plain(stream) => stream.quit().await,
                Self::Tls(stream) => stream.quit().await,
            }
            .map_err(|error| anyhow!("ftp quit failed: {}", error))
        })
        .await
        .map_err(|error| anyhow!("ftp quit timed out: {}", error))?
    }
}

#[async_trait]
impl ClientTransfer for ClientTransferFtp {
    type Event = RespEventSftp;
    type Context = ContextFtp;

    fn get_id(&self) -> String {
        self.client_transfer_id.clone()
    }

    async fn close(&self) -> Result<()> {
        let client = tokio::time::timeout(FTP_CONTROL_TIMEOUT, async { self.state.lock().await.take() }).await;
        let mut client = match client {
            Ok(Some(client)) => client,
            Ok(None) => return Ok(()),
            Err(error) => {
                warn!("[ftp] close waiting for active operation timed out: {}", error);
                return Err(anyhow!("FTP close waiting for active operation timed out: {}", error));
            }
        };
        if let Err(error) = client.quit().await {
            warn!("[ftp] close failed: {}", error);
            return Err(error);
        }
        Ok(())
    }

    async fn home_dir(&self) -> Result<String> {
        let mut guard = self.state.lock().await;
        let client = guard.as_mut().ok_or_else(|| anyhow!("ftp not opened"))?;
        Ok(path_util::normalize_to_slash(client.pwd().await?.as_str()))
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let mut guard = self.state.lock().await;
        let client = guard.as_mut().ok_or_else(|| anyhow!("ftp not opened"))?;
        Self::entry_exists(client, path).await
    }

    fn parent_path(&self, path: &str) -> Option<String> {
        Self::parent_of(path)
    }

    async fn realpath(&self, path: &str) -> Result<String> {
        let mut guard = self.state.lock().await;
        let client = guard.as_mut().ok_or_else(|| anyhow!("ftp not opened"))?;
        let origin = client.pwd().await?;
        client.cwd(path).await?;
        let resolved = client.pwd().await?;
        client.cwd(origin.as_str()).await?;
        Ok(path_util::normalize_to_slash(resolved.as_str()))
    }

    async fn read_link(&self, path: &str) -> Result<String> {
        Err(anyhow!("ftp does not support symlink: {}", path))
    }

    async fn rename(&self, from: &str, to: &str) -> Result<()> {
        let mut guard = self.state.lock().await;
        let client = guard.as_mut().ok_or_else(|| anyhow!("ftp not opened"))?;
        client.rename(from, to).await
    }

    async fn chmod(&self, _paths: &[String], _mode: u32) -> Result<()> {
        Err(anyhow!("ftp does not support chmod"))
    }

    async fn list_dir(&self, path: &str) -> Result<Vec<FileEntry>> {
        let mut guard = self.state.lock().await;
        let client = guard.as_mut().ok_or_else(|| anyhow!("ftp not opened"))?;
        Self::list_entries(client, path).await
    }

    async fn stat(&self, path: &str, _kind: &FileKind) -> Result<FileEntry> {
        if path == "/" || path.is_empty() {
            return Ok(FileEntry {
                hidden: false,
                name: "/".to_string(),
                path: "/".to_string(),
                kind: FileKind::Directory,
                size: None,
                uid: None,
                gid: None,
                user: None,
                group: None,
                permissions: None,
                mtime: None,
            });
        }
        let mut guard = self.state.lock().await;
        let client = guard.as_mut().ok_or_else(|| anyhow!("ftp not opened"))?;
        let parent = Self::parent_of(path).ok_or_else(|| anyhow!("ftp stat parent missing: {}", path))?;
        let name = Self::posix_name(path);
        let entries = Self::list_entries(client, parent.as_str()).await?;
        entries.into_iter().find(|entry| entry.name == name).ok_or_else(|| anyhow!("ftp stat not found: {}", path))
    }

    async fn open_read(&self, path: &str) -> Result<Pin<Box<dyn AsyncRead + Send>>> {
        let (reader, mut writer) = tokio::io::duplex(65536);
        let state = Arc::clone(&self.state);
        let path = path.to_string();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Result<()>>();
        tokio::spawn(async move {
            let mut guard = state.lock().await;
            let Some(client) = guard.as_mut() else {
                let _ = ready_tx.send(Err(anyhow!("ftp not opened")));
                return;
            };
            match client {
                FtpClient::Plain(stream) => match stream.retr_as_stream(path.as_str()).await {
                    Ok(mut data) => {
                        let _ = ready_tx.send(Ok(()));
                        let _ = tokio::io::copy(&mut data, &mut writer).await;
                        let _ = data.finish().await;
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(anyhow!("ftp retr failed: {}", error)));
                    }
                },
                FtpClient::Tls(stream) => match stream.retr_as_stream(path.as_str()).await {
                    Ok(mut data) => {
                        let _ = ready_tx.send(Ok(()));
                        let _ = tokio::io::copy(&mut data, &mut writer).await;
                        let _ = data.finish().await;
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(anyhow!("ftp retr failed: {}", error)));
                    }
                },
            }
        });
        ready_rx.await.map_err(|error| anyhow!("ftp retr join failed: {}", error))??;
        Ok(Box::pin(reader))
    }

    async fn create(&self, path: &str, kind: FileKind, _link_target: Option<&str>) -> Result<()> {
        let mut guard = self.state.lock().await;
        let client = guard.as_mut().ok_or_else(|| anyhow!("ftp not opened"))?;
        match kind {
            FileKind::Directory => Self::create_dir_all(client, path).await,
            FileKind::File => {
                if let Some(parent) = Self::parent_of(path) {
                    Self::create_dir_all(client, parent.as_str()).await?;
                }
                match client {
                    FtpClient::Plain(stream) => {
                        let data = stream.put_with_stream(path).await.map_err(|error| anyhow!("ftp create file failed: {}", error))?;
                        data.finish().await.map_err(|error| anyhow!("ftp create file finish failed: {}", error))?;
                    }
                    FtpClient::Tls(stream) => {
                        let data = stream.put_with_stream(path).await.map_err(|error| anyhow!("ftp create file failed: {}", error))?;
                        data.finish().await.map_err(|error| anyhow!("ftp create file finish failed: {}", error))?;
                    }
                }
                Ok(())
            }
            FileKind::Symlink => Err(anyhow!("ftp does not support symlink")),
            FileKind::Other => Err(anyhow!("cannot create entry of kind Other")),
            FileKind::Parent => Err(anyhow!("cannot create entry of kind Parent")),
        }
    }

    async fn stat_properties(&self, path: &str, kind: &FileKind) -> Result<FileStatProperties> {
        let entry = self.stat(path, kind).await?;
        if *kind != FileKind::Directory {
            return Ok(FileStatProperties { path: path_util::normalize_to_slash(path), entry, dir: None });
        }
        let mut guard = self.state.lock().await;
        let client = guard.as_mut().ok_or_else(|| anyhow!("ftp not opened"))?;
        let dir = Self::calc_dir_properties(client, path).await?;
        Ok(FileStatProperties { path: path_util::normalize_to_slash(path), entry, dir: Some(dir) })
    }

    async fn open(&self, context: &Self::Context, handler: EventHandler<Self::Event>) -> Result<()> {
        let host = context.device.read().host.clone();
        if host.is_empty() {
            return Err(anyhow!("ftp host empty"));
        }
        let config = context.config.read().clone();
        let base = config.get_base()?;
        if base.port == 0 {
            return Err(anyhow!("ftp port invalid"));
        }
        let username = base.username.clone().ok_or_else(|| anyhow!("ftp username missing"))?;
        let password = base.password.clone().ok_or_else(|| anyhow!("ftp password missing"))?;
        let timeout = base.connect_timeout_seconds.filter(|v| *v > 0).map(Duration::from_secs);
        let security = config.security;
        let ignore_cert = config.ignore_cert;
        let mut client = tokio::time::timeout(timeout.unwrap_or(FTP_CONTROL_TIMEOUT), Self::connect_client(host.as_str(), base.port, timeout, security, ignore_cert))
            .await
            .map_err(|error| anyhow!("FTP connect or welcome timed out: {}", error))??;
        client.login(username.as_str(), password.as_str()).await?;
        client.set_binary().await?;
        let _ = handler(RespEventSftp::OnOpenStep(RespSftpOpenStep {
            client_transfer_id: self.client_transfer_id.clone(),
            step: OpenSshStep { progress: 100.0, step_type: RespSshOpenStepType::SessionOpened, data: None },
        }))
        .await;
        let mut guard = self.state.lock().await;
        *guard = Some(client);
        Ok(())
    }

    async fn list_dir_for_transfer_plan(&self, path: &str, cancel: Option<&CancelNotify<'_>>) -> Result<Vec<FileEntry>> {
        let list_entries = || async { self.list_dir(path).await };
        match cancel {
            Some(notify) => await_with_cancel(notify, list_entries).await,
            None => list_entries().await,
        }
    }

    async fn delete(&self, entries: &[PlannedDelete], cancel: Option<&CancelNotify<'_>>, handler: RespSftpDeleteProgressHandler) -> Result<()> {
        let mut guard = self.state.lock().await;
        let client = guard.as_mut().ok_or_else(|| anyhow!("ftp not opened"))?;
        let files_total = entries.len() as u32;
        let mut files_done: u32 = 0;
        for entry in entries {
            if let Some(notify) = cancel {
                if notify.is_cancelled() {
                    return Err(cancelled());
                }
            }
            if entry.kind == FileKind::Directory {
                client.rmdir(entry.path.as_str()).await?;
            } else {
                client.rm(entry.path.as_str()).await?;
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
        let (mut reader, writer) = tokio::io::duplex(65536);
        let state = Arc::clone(&self.state);
        let path = path.to_string();
        let progress_path = path.clone();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel::<Result<()>>();
        tokio::spawn(async move {
            let mut guard = state.lock().await;
            let Some(client) = guard.as_mut() else {
                let _ = ready_tx.send(Err(anyhow!("ftp not opened")));
                return;
            };
            if let Some(parent) = ClientTransferFtp::parent_of(path.as_str()) {
                if let Err(error) = ClientTransferFtp::create_dir_all(client, parent.as_str()).await {
                    let _ = ready_tx.send(Err(error));
                    return;
                }
            }
            if offset.is_none() && !overwrite {
                match ClientTransferFtp::entry_exists(client, path.as_str()).await {
                    Ok(true) => {
                        let _ = ready_tx.send(Err(anyhow!("ftp target exists: {}", path)));
                        return;
                    }
                    Ok(false) => {}
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                }
            }
            if let Some(pos) = offset {
                if let Err(error) = client.resume(pos).await {
                    let _ = ready_tx.send(Err(error));
                    return;
                }
            }
            match client {
                FtpClient::Plain(stream) => match stream.put_with_stream(path.as_str()).await {
                    Ok(mut data) => {
                        let _ = ready_tx.send(Ok(()));
                        let _ = tokio::io::copy(&mut reader, &mut data).await;
                        let _ = data.finish().await;
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(anyhow!("ftp stor failed: {}", error)));
                    }
                },
                FtpClient::Tls(stream) => match stream.put_with_stream(path.as_str()).await {
                    Ok(mut data) => {
                        let _ = ready_tx.send(Ok(()));
                        let _ = tokio::io::copy(&mut reader, &mut data).await;
                        let _ = data.finish().await;
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(anyhow!("ftp stor failed: {}", error)));
                    }
                },
            }
        });
        ready_rx.await.map_err(|error| anyhow!("ftp stor join failed: {}", error))??;
        Ok(Box::pin(ProgressWriter::new(Box::pin(writer), progress_path, total, offset.unwrap_or(0), handler)))
    }
}
