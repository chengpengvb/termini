use std::pin::Pin;
use crate::core::tpkt;
use crate::crypto::tls;
use std::time::Duration;
use tokio::time::timeout;
use tokio::net::TcpStream;
use tokio_openssl::SslStream;
use std::task::{Context, Poll};
use socket2::{SockRef, TcpKeepalive};
use crate::core::error::{RdpProtocolError, RdpResult};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf, ReadHalf, WriteHalf};

pub const FASTPATH_ACTION_X224: u8 = 0x03;

const TERMINATED_LIMIT: usize = 64 * 1024;

pub enum RdpStream {
    Plain(TcpStream),
    Tls(Box<SslStream<TcpStream>>),
}

impl AsyncRead for RdpStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            RdpStream::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            RdpStream::Tls(stream) => Pin::new(stream.as_mut()).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for RdpStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            RdpStream::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            RdpStream::Tls(stream) => Pin::new(stream.as_mut()).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            RdpStream::Plain(stream) => Pin::new(stream).poll_flush(cx),
            RdpStream::Tls(stream) => Pin::new(stream.as_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            RdpStream::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            RdpStream::Tls(stream) => Pin::new(stream.as_mut()).poll_shutdown(cx),
        }
    }
}

pub enum RdpFrame {
    X224(Vec<u8>),
    FastPath { flags: u8, payload: Vec<u8> },
}

pub enum TlsUpgradeResult {
    Established { transport: RdpTransport, public_key: Vec<u8>, fingerprint: String },
    Untrusted { fingerprint: String },
}

pub struct RdpTransport {
    stream: RdpStream,
}

impl RdpTransport {
    pub async fn connect(host: &str, port: u16, connect_timeout_seconds: u64, keepalive_interval_seconds: u64) -> RdpResult<Self> {
        let connect_future = TcpStream::connect((host, port));
        let stream = match timeout(Duration::from_secs(connect_timeout_seconds), connect_future).await {
            Ok(Ok(stream)) => stream,
            Ok(Err(error)) => return Err(RdpProtocolError::protocol(format!("failed to connect to [{}:{}]: {}", host, port, error))),
            Err(_) => return Err(RdpProtocolError::protocol(format!("connection to [{}:{}] timed out", host, port))),
        };
        stream.set_nodelay(true).map_err(|error| RdpProtocolError::protocol(format!("failed to set TCP_NODELAY: {}", error)))?;
        if keepalive_interval_seconds > 0 {
            let seconds = Duration::from_secs(keepalive_interval_seconds);
            let keepalive = TcpKeepalive::new().with_time(seconds).with_interval(seconds);
            #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd", target_vendor = "apple"))]
            let keepalive = keepalive.with_retries(3);
            SockRef::from(&stream).set_tcp_keepalive(&keepalive).map_err(|error| RdpProtocolError::protocol(format!("failed to set TCP keepalive: {}", error)))?;
        }
        Ok(Self { stream: RdpStream::Plain(stream) })
    }

    pub async fn upgrade_tls(self, host: &str, ignore_cert: bool, trusted_sha256: Option<&str>) -> RdpResult<TlsUpgradeResult> {
        let RdpStream::Plain(plain) = self.stream else {
            return Err(RdpProtocolError::protocol("TLS is already established; cannot upgrade again".to_string()));
        };
        match tls::upgrade(plain, host, ignore_cert, trusted_sha256).await? {
            tls::TlsOutcome::Established(upgrade) => {
                Ok(TlsUpgradeResult::Established { transport: Self { stream: RdpStream::Tls(Box::new(upgrade.stream)) }, public_key: upgrade.public_key, fingerprint: upgrade.fingerprint })
            }
            tls::TlsOutcome::Untrusted { fingerprint } => Ok(TlsUpgradeResult::Untrusted { fingerprint }),
        }
    }

    pub async fn send(&mut self, data: &[u8]) -> RdpResult<()> {
        self.stream.write_all(data).await.map_err(|error| RdpProtocolError::protocol(format!("failed to send data: {}", error)))?;
        self.stream.flush().await.map_err(|error| RdpProtocolError::protocol(format!("failed to flush data: {}", error)))
    }

    pub async fn read_frame(&mut self) -> RdpResult<RdpFrame> {
        read_frame(&mut self.stream).await
    }

    pub async fn read_x224(&mut self) -> RdpResult<Vec<u8>> {
        match self.read_frame().await? {
            RdpFrame::X224(payload) => Ok(payload),
            RdpFrame::FastPath { .. } => Err(RdpProtocolError::protocol("received fastpath frame during connection phase".to_string())),
        }
    }

    pub async fn read_der(&mut self) -> RdpResult<Vec<u8>> {
        read_der(&mut self.stream).await
    }

    pub async fn read_u32_le(&mut self) -> RdpResult<u32> {
        let mut raw = [0u8; 4];
        read_exact(&mut self.stream, &mut raw).await?;
        Ok(u32::from_le_bytes(raw))
    }

    pub async fn read_terminated(&mut self) -> RdpResult<Vec<u8>> {
        let mut payload = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            read_exact(&mut self.stream, &mut byte).await?;
            if byte[0] == 0 {
                return Ok(payload);
            }
            payload.push(byte[0]);
            if payload.len() > TERMINATED_LIMIT {
                return Err(RdpProtocolError::protocol(format!("NUL-terminated message too long: {}", payload.len())));
            }
        }
    }

    pub async fn read_bytes(&mut self, length: usize) -> RdpResult<Vec<u8>> {
        let mut raw = vec![0u8; length];
        read_exact(&mut self.stream, &mut raw).await?;
        Ok(raw)
    }

    pub fn split(self) -> (ReadHalf<RdpStream>, WriteHalf<RdpStream>) {
        tokio::io::split(self.stream)
    }
}

pub async fn read_frame<R: AsyncRead + Unpin>(reader: &mut R) -> RdpResult<RdpFrame> {
    let mut header = [0u8; 1];
    read_exact(reader, &mut header).await?;
    if header[0] & 0x03 == FASTPATH_ACTION_X224 {
        let mut rest = [0u8; 3];
        read_exact(reader, &mut rest).await?;
        let length = u16::from_be_bytes([rest[1], rest[2]]) as usize;
        if length < tpkt::TPKT_HEADER_LENGTH {
            return Err(RdpProtocolError::protocol(format!("tpkt length too small: {}", length)));
        }
        let mut payload = vec![0u8; length - tpkt::TPKT_HEADER_LENGTH];
        read_exact(reader, &mut payload).await?;
        return Ok(RdpFrame::X224(payload));
    }

    let mut length_byte = [0u8; 1];
    read_exact(reader, &mut length_byte).await?;
    let (total, consumed) = match length_byte[0] & 0x80 != 0 {
        true => {
            let mut second = [0u8; 1];
            read_exact(reader, &mut second).await?;
            ((((length_byte[0] & 0x7F) as usize) << 8) | second[0] as usize, 3usize)
        }
        false => (length_byte[0] as usize, 2usize),
    };
    if total < consumed {
        return Err(RdpProtocolError::protocol(format!("fastpath length too small: {}", total)));
    }
    let mut payload = vec![0u8; total - consumed];
    read_exact(reader, &mut payload).await?;
    Ok(RdpFrame::FastPath { flags: (header[0] >> 6) & 0x03, payload })
}

pub async fn read_der<R: AsyncRead + Unpin>(reader: &mut R) -> RdpResult<Vec<u8>> {
    let mut prefix = [0u8; 2];
    read_exact(reader, &mut prefix).await?;
    let mut message = prefix.to_vec();
    let content_length = if prefix[1] < 0x80 {
        prefix[1] as usize
    } else {
        let count = (prefix[1] & 0x7F) as usize;
        if count == 0 || count > 4 {
            return Err(RdpProtocolError::encoding(format!("der length octet count invalid: {}", count)));
        }
        let mut length_bytes = vec![0u8; count];
        read_exact(reader, &mut length_bytes).await?;
        message.extend_from_slice(&length_bytes);
        length_bytes.iter().fold(0usize, |acc, byte| (acc << 8) | *byte as usize)
    };
    let mut content = vec![0u8; content_length];
    read_exact(reader, &mut content).await?;
    message.extend_from_slice(&content);
    Ok(message)
}

async fn read_exact<R: AsyncRead + Unpin>(reader: &mut R, buffer: &mut [u8]) -> RdpResult<()> {
    reader.read_exact(buffer).await.map_err(|error| RdpProtocolError::protocol(format!("failed to read data: {}", error)))?;
    Ok(())
}
