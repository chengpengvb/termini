use std::pin::Pin;
use russh::client::Msg;
use russh::ChannelStream;
use tokio::net::TcpStream;
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

pub enum SshTransport {
    Tcp(TcpStream),
    Jump(ChannelStream<Msg>),
}

impl AsyncRead for SshTransport {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            SshTransport::Tcp(stream) => Pin::new(stream).poll_read(cx, buf),
            SshTransport::Jump(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for SshTransport {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            SshTransport::Tcp(stream) => Pin::new(stream).poll_write(cx, buf),
            SshTransport::Jump(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            SshTransport::Tcp(stream) => Pin::new(stream).poll_flush(cx),
            SshTransport::Jump(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            SshTransport::Tcp(stream) => Pin::new(stream).poll_shutdown(cx),
            SshTransport::Jump(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
