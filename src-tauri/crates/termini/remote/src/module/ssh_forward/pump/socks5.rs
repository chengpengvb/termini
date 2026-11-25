use tokio::net::TcpStream;
use anyhow::{bail, Result};
use std::net::{Ipv4Addr, Ipv6Addr};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const VERSION: u8 = 0x05;
const METHOD_NO_AUTH: u8 = 0x00;
const CMD_CONNECT: u8 = 0x01;
const ATYP_IPV4: u8 = 0x01;
const ATYP_DOMAIN: u8 = 0x03;
const ATYP_IPV6: u8 = 0x04;
const REP_SUCCEEDED: u8 = 0x00;
const REP_GENERAL_FAILURE: u8 = 0x01;
const REP_CMD_NOT_SUPPORTED: u8 = 0x07;

pub struct Socks5Target {
    pub host: String,
    pub port: u16,
}

pub async fn handshake(stream: &mut TcpStream) -> Result<Socks5Target> {
    let version = stream.read_u8().await?;
    if version != VERSION {
        bail!("socks5 unsupported version: {}", version);
    }
    let method_count = stream.read_u8().await? as usize;
    let mut methods = vec![0u8; method_count];
    stream.read_exact(&mut methods).await?;
    stream.write_all(&[VERSION, METHOD_NO_AUTH]).await?;

    let mut head = [0u8; 4];
    stream.read_exact(&mut head).await?;
    if head[0] != VERSION {
        bail!("socks5 request unsupported version: {}", head[0]);
    }
    if head[1] != CMD_CONNECT {
        reply(stream, REP_CMD_NOT_SUPPORTED).await?;
        bail!("socks5 unsupported command: {}", head[1]);
    }

    let host = match head[3] {
        ATYP_IPV4 => {
            let mut addr = [0u8; 4];
            stream.read_exact(&mut addr).await?;
            Ipv4Addr::from(addr).to_string()
        }
        ATYP_DOMAIN => {
            let len = stream.read_u8().await? as usize;
            let mut domain = vec![0u8; len];
            stream.read_exact(&mut domain).await?;
            String::from_utf8(domain).map_err(|_| anyhow::anyhow!("socks5 invalid domain"))?
        }
        ATYP_IPV6 => {
            let mut addr = [0u8; 16];
            stream.read_exact(&mut addr).await?;
            Ipv6Addr::from(addr).to_string()
        }
        other => {
            reply(stream, REP_CMD_NOT_SUPPORTED).await?;
            bail!("socks5 unsupported address type: {}", other);
        }
    };
    let port = stream.read_u16().await?;
    Ok(Socks5Target { host, port })
}

pub async fn reply_succeeded(stream: &mut TcpStream) -> Result<()> {
    reply(stream, REP_SUCCEEDED).await
}

pub async fn reply_failed(stream: &mut TcpStream) -> Result<()> {
    reply(stream, REP_GENERAL_FAILURE).await
}

async fn reply(stream: &mut TcpStream, code: u8) -> Result<()> {
    stream.write_all(&[VERSION, code, 0x00, ATYP_IPV4, 0, 0, 0, 0, 0, 0]).await?;
    Ok(())
}
