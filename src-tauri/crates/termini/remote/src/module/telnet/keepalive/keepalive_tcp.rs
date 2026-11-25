use std::time::Duration;
use tokio::net::TcpStream;
use socket2::{SockRef, TcpKeepalive};

pub fn apply_tcp_keepalive(stream: &TcpStream, interval_seconds: u64) -> anyhow::Result<()> {
    if interval_seconds == 0 {
        return Ok(());
    }
    let seconds = Duration::from_secs(interval_seconds);
    let ka = TcpKeepalive::new().with_time(seconds).with_interval(seconds);
    #[cfg(any(target_os = "linux", target_os = "android", target_os = "freebsd", target_vendor = "apple"))]
    let ka = ka.with_retries(3);
    SockRef::from(stream).set_tcp_keepalive(&ka)?;
    Ok(())
}
