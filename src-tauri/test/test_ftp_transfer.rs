use std::sync::Arc;
use crate::model::device::Device;
use crate::model::dto::shared_ptr::Shared;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::module::common::client_terminal::EventHandler;
use crate::module::sftp::client_transfer::ClientTransfer;
use crate::module::common::context::context_ftp::ContextFtp;
use crate::module::common::context::context_base::WindowSize;
use crate::model::resp::event::resp_event_sftp::RespEventSftp;
use crate::module::sftp::client_transfer_ftp::ClientTransferFtp;
use crate::module::common::dto::file_entry::{FileKind, PlannedDelete};
use crate::model::resp::resp_sftp::{RespSftpDeleteProgressHandler, RespSftpTransferProgressHandler};
use crate::model::config_ftp::{ConfigFtp, FTP_SECURITY_EXPLICIT, FTP_SECURITY_IMPLICIT, FTP_SECURITY_NONE};

fn context(port: u16, security: i16, ignore_cert: bool) -> ContextFtp {
    let mut device = Device::default();
    device.host = "127.0.0.1".to_string();
    let mut config = ConfigFtp::default();
    config.security = security;
    config.ignore_cert = ignore_cert;
    let mut base = config.get_base().expect("ftp base");
    base.port = port;
    base.username = Some("root".to_string());
    base.password = Some("123456".to_string());
    base.connect_timeout_seconds = Some(8);
    config.set_base(base).expect("set ftp base");
    ContextFtp { size: WindowSize::default(), config: Shared::new(config), device: Shared::new(device) }
}

fn event() -> EventHandler<RespEventSftp> {
    Arc::new(|_| Box::pin(async { Ok(()) }))
}

fn tf() -> RespSftpTransferProgressHandler {
    Arc::new(|_| Box::pin(async { Ok(()) }))
}

fn del() -> RespSftpDeleteProgressHandler {
    Arc::new(|_| Box::pin(async { Ok(()) }))
}

async fn open_ftp(port: u16, security: i16, ignore_cert: bool) -> ClientTransferFtp {
    let client = ClientTransferFtp::new();
    client.open(&context(port, security, ignore_cert), event()).await.expect("ftp open");
    client
}

async fn upload(client: &ClientTransferFtp, path: &str, body: &[u8]) {
    let mut writer = client.open_write(path, None, true, Some(body.len() as u64), tf()).await.expect("open_write");
    writer.write_all(body).await.expect("write");
    writer.shutdown().await.expect("shutdown");
}

async fn download(client: &ClientTransferFtp, path: &str) -> Vec<u8> {
    let mut reader = client.open_read(path).await.expect("open_read");
    let mut buf = Vec::new();
    reader.read_to_end(&mut buf).await.expect("read");
    buf
}

async fn roundtrip(client: &ClientTransferFtp) {
    let utf8 = client.list_dir("/root/ftp-sample/utf8").await.expect("list utf8");
    assert!(utf8.iter().any(|entry| entry.name.contains("中文文件")), "utf8 中文文件 missing: {:?}", utf8.iter().map(|entry| entry.name.clone()).collect::<Vec<_>>());
    let gbk = client.list_dir("/root/ftp-sample/gbk").await.expect("list gbk");
    assert!(!gbk.is_empty(), "gbk list empty");
    let remote = "/root/ftp-sample/termini-ftp-test.bin";
    upload(client, remote, b"ftp-roundtrip").await;
    assert_eq!(download(client, remote).await, b"ftp-roundtrip");
    let renamed = "/root/ftp-sample/termini-ftp-test-renamed.bin";
    client.rename(remote, renamed).await.expect("rename");
    assert!(client.chmod(&[renamed.to_string()], 0o644).await.expect_err("chmod").to_string().contains("does not support chmod"));
    assert!(client.read_link(renamed).await.expect_err("read_link").to_string().contains("does not support symlink"));
    client.delete(&[PlannedDelete { path: renamed.to_string(), kind: FileKind::File }], None, del()).await.expect("delete");
    assert!(!client.exists(renamed).await.expect("exists"));
    client.close().await.expect("close");
    assert!(client.list_dir("/").await.is_err(), "close 后 session 应空");
}

#[tokio::test]
async fn ftp_all_cases() {
    let cert = ClientTransferFtp::new().open(&context(990, FTP_SECURITY_IMPLICIT, false), event()).await;
    assert!(cert.is_err(), "自签证书不忽略应失败");
    for (port, security, ignore_cert) in [(21, FTP_SECURITY_NONE, false), (21, FTP_SECURITY_EXPLICIT, true), (990, FTP_SECURITY_IMPLICIT, true)] {
        let client = tokio::time::timeout(std::time::Duration::from_secs(15), open_ftp(port, security, ignore_cert)).await.expect("ftp open timeout");
        let home = tokio::time::timeout(std::time::Duration::from_secs(15), client.home_dir()).await.expect("home timeout").expect("home");
        assert!(!home.is_empty(), "home empty");
        tokio::time::timeout(std::time::Duration::from_secs(30), roundtrip(&client)).await.expect("roundtrip timeout");
        tokio::time::timeout(std::time::Duration::from_secs(15), client.open(&context(port, security, ignore_cert), event())).await.expect("reopen timeout").expect("reopen");
        tokio::time::timeout(std::time::Duration::from_secs(15), client.close()).await.expect("close reopen timeout").expect("close reopen");
    }
}
