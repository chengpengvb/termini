use std::sync::Arc;
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use tokio::sync::Mutex as AsyncMutex;
use russh::keys::{Algorithm, PrivateKey};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use russh::{client, server, ChannelOpenFailure};
use crate::module::ssh_forward::pump::{forward_pump, socks5};
use russh::server::{Auth, Msg as ServerMsg, Server as _, Session};

struct ClientHandler;

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(&mut self, _key: &russh::keys::PublicKey) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

#[derive(Clone)]
struct DirectTcpipServer {
    delay_before_accept: Arc<AsyncMutex<Duration>>,
    force_fail: Arc<AsyncMutex<bool>>,
    last_target: Arc<AsyncMutex<Option<(String, u32)>>>,
}

struct DirectTcpipHandler {
    delay_before_accept: Arc<AsyncMutex<Duration>>,
    force_fail: Arc<AsyncMutex<bool>>,
    last_target: Arc<AsyncMutex<Option<(String, u32)>>>,
}

impl server::Server for DirectTcpipServer {
    type Handler = DirectTcpipHandler;

    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self::Handler {
        DirectTcpipHandler { delay_before_accept: self.delay_before_accept.clone(), force_fail: self.force_fail.clone(), last_target: self.last_target.clone() }
    }
}

impl server::Handler for DirectTcpipHandler {
    type Error = russh::Error;

    async fn auth_password(&mut self, _user: &str, _password: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_publickey(&mut self, _user: &str, _key: &russh::keys::PublicKey) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn channel_open_session(&mut self, _channel: russh::Channel<ServerMsg>, reply: server::ChannelOpenHandle, _session: &mut Session) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }

    async fn channel_open_direct_tcpip(
        &mut self,
        channel: russh::Channel<ServerMsg>,
        host_to_connect: &str,
        port_to_connect: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: server::ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        {
            let mut guard = self.last_target.lock().await;
            *guard = Some((host_to_connect.to_string(), port_to_connect));
        }
        let delay = *self.delay_before_accept.lock().await;
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        if *self.force_fail.lock().await {
            reply.reject(ChannelOpenFailure::ConnectFailed).await;
            return Ok(());
        }
        let mut tcp = match TcpStream::connect((host_to_connect, port_to_connect as u16)).await {
            Ok(tcp) => tcp,
            Err(_) => {
                reply.reject(ChannelOpenFailure::ConnectFailed).await;
                return Ok(());
            }
        };
        reply.accept().await;
        tokio::spawn(async move {
            let mut stream = channel.into_stream();
            let _ = tokio::io::copy_bidirectional(&mut stream, &mut tcp).await;
        });
        Ok(())
    }
}

async fn start_http_target(body: &'static str) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind http");
    let addr = listener.local_addr().expect("http addr");
    tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else { break };
            let mut buf = [0u8; 2048];
            let _ = sock.read(&mut buf).await;
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            let _ = sock.write_all(response.as_bytes()).await;
        }
    });
    addr
}

async fn open_ssh_client(server: DirectTcpipServer) -> (client::Handle<ClientHandler>, tokio::task::JoinHandle<()>) {
    let key = PrivateKey::random(&mut rand10::rng(), Algorithm::Ed25519).expect("ssh key");
    let config = Arc::new(server::Config { keys: vec![key], ..Default::default() });
    let mut server = server;
    let ssh_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind ssh");
    let ssh_addr = ssh_listener.local_addr().expect("ssh addr");
    let server_task = tokio::spawn(async move {
        let run = server.run_on_socket(config, &ssh_listener);
        run.await.ok();
    });
    let mut handle = client::connect(Arc::new(client::Config::default()), ssh_addr, ClientHandler).await.expect("ssh connect");
    let auth = handle.authenticate_password("test", "test").await.expect("auth");
    assert!(auth.success());
    (handle, server_task)
}

async fn spawn_production_dynamic_proxy(handle: Arc<client::Handle<ClientHandler>>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind socks");
    let port = listener.local_addr().expect("socks addr").port();
    tokio::spawn(async move {
        loop {
            let Ok((tcp, peer)) = listener.accept().await else { break };
            let handle = handle.clone();
            tokio::spawn(async move {
                let mut tcp = tcp;
                let target = match socks5::handshake(&mut tcp).await {
                    Ok(target) => target,
                    Err(_) => return,
                };
                let channel = match handle.channel_open_direct_tcpip(target.host, target.port as u32, peer.ip().to_string(), peer.port() as u32).await {
                    Ok(channel) => channel,
                    Err(_) => {
                        let _ = socks5::reply_failed(&mut tcp).await;
                        return;
                    }
                };
                if socks5::reply_succeeded(&mut tcp).await.is_err() {
                    return;
                }
                forward_pump::pump(channel, tcp).await;
            });
        }
    });
    port
}

async fn spawn_buggy_early_success_proxy(handle: Arc<client::Handle<ClientHandler>>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind socks");
    let port = listener.local_addr().expect("socks addr").port();
    tokio::spawn(async move {
        loop {
            let Ok((tcp, peer)) = listener.accept().await else { break };
            let handle = handle.clone();
            tokio::spawn(async move {
                let mut tcp = tcp;
                let target = match socks5::handshake(&mut tcp).await {
                    Ok(target) => target,
                    Err(_) => return,
                };
                if socks5::reply_succeeded(&mut tcp).await.is_err() {
                    return;
                }
                let channel = match handle.channel_open_direct_tcpip(target.host, target.port as u32, peer.ip().to_string(), peer.port() as u32).await {
                    Ok(channel) => channel,
                    Err(_) => return,
                };
                forward_pump::pump(channel, tcp).await;
            });
        }
    });
    port
}

async fn socks5_connect_ipv4(proxy_port: u16, dest: SocketAddr) -> std::io::Result<TcpStream> {
    let mut stream = TcpStream::connect(("127.0.0.1", proxy_port)).await?;
    stream.write_all(&[0x05, 0x01, 0x00]).await?;
    let mut method = [0u8; 2];
    stream.read_exact(&mut method).await?;
    if method != [0x05, 0x00] {
        return Err(std::io::Error::other(format!("socks method {:?}", method)));
    }
    let ip = match dest.ip() {
        std::net::IpAddr::V4(v4) => v4.octets(),
        _ => return Err(std::io::Error::other("need ipv4 dest")),
    };
    let port = dest.port().to_be_bytes();
    let mut req = vec![0x05, 0x01, 0x00, 0x01];
    req.extend_from_slice(&ip);
    req.extend_from_slice(&port);
    stream.write_all(&req).await?;
    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await?;
    if reply[1] != 0x00 {
        return Err(std::io::Error::other(format!("socks reply status {}", reply[1])));
    }
    Ok(stream)
}

async fn http_get_over(mut stream: TcpStream) -> String {
    stream.write_all(b"GET / HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n").await.expect("write");
    let mut buf = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(3), stream.read_to_end(&mut buf)).await;
    String::from_utf8_lossy(&buf).to_string()
}

#[tokio::test]
async fn socks5_handshake_does_not_emit_connect_success() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.expect("accept");
        let target = socks5::handshake(&mut sock).await.expect("handshake");
        assert_eq!(target.host, "1.2.3.4");
        assert_eq!(target.port, 443);
        tokio::time::sleep(Duration::from_millis(200)).await;
        sock
    });
    let client = tokio::spawn(async move {
        let mut stream = TcpStream::connect(addr).await.expect("connect");
        stream.write_all(&[0x05, 0x01, 0x00]).await.expect("method");
        let mut method = [0u8; 2];
        stream.read_exact(&mut method).await.expect("method reply");
        assert_eq!(method, [0x05, 0x00]);
        stream.write_all(&[0x05, 0x01, 0x00, 0x01, 1, 2, 3, 4, 0x01, 0xbb]).await.expect("connect req");
        let result = tokio::time::timeout(Duration::from_millis(100), async {
            let mut reply = [0u8; 10];
            stream.read_exact(&mut reply).await.map(|_| reply)
        })
        .await;
        assert!(result.is_err(), "handshake 返回后不应已发出 CONNECT 成功应答，否则 curl 会提前发 TLS 然后卡住");
    });
    client.await.expect("client");
    server.abort();
}

#[tokio::test]
async fn shared_session_can_still_open_direct_tcpip() {
    let marker = "TERMIX-SHARED-OK";
    let http_addr = start_http_target(marker).await;
    let delay = Arc::new(AsyncMutex::new(Duration::ZERO));
    let force_fail = Arc::new(AsyncMutex::new(false));
    let last_target = Arc::new(AsyncMutex::new(None));
    let (handle, server_task) = open_ssh_client(DirectTcpipServer { delay_before_accept: delay, force_fail, last_target }).await;
    let handle = Arc::new(handle);

    // 模拟“终端”已在这条 SSH 上开了一个会话通道并保持
    let terminal_channel = handle.channel_open_session().await.expect("终端会话通道应能打开");
    let _keep = terminal_channel;

    // 复用同一条 SSH：转发再开 direct-tcpip
    let proxy_port = spawn_production_dynamic_proxy(handle.clone()).await;
    tokio::time::sleep(Duration::from_millis(30)).await;
    let stream = socks5_connect_ipv4(proxy_port, http_addr).await.expect("复用会话应能建 direct-tcpip");
    let body = http_get_over(stream).await;
    assert!(body.contains(marker), "同一条 SSH 上同时存在会话通道时，direct-tcpip 仍应可用, body={}", body);

    server_task.abort();
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "EN").await;
}

#[tokio::test]
async fn stale_shared_handle_is_not_closed_but_direct_tcpip_stalls() {
    let marker = "x";
    let http_addr = start_http_target(marker).await;
    // 服务端 TCP 仍在，但对新开通道请求长时间不回应答（模拟 NAT/空闲后连接“半开/失联”，
    // 对端已不再服务本连接的 channel-open，但没有发 FIN/RST）
    let delay = Arc::new(AsyncMutex::new(Duration::from_secs(30)));
    let force_fail = Arc::new(AsyncMutex::new(false));
    let last_target = Arc::new(AsyncMutex::new(None));
    let (handle, server_task) = open_ssh_client(DirectTcpipServer { delay_before_accept: delay, force_fail, last_target }).await;
    let handle = Arc::new(handle);

    // 复用判活门只看 is_closed()：这条“失联”连接下它仍是 false，于是会被当成可复用
    assert!(!handle.is_closed(), "失联连接下 is_closed() 仍为 false —— 这正是共享复用的盲区");

    // 在这条“看起来没关”的连接上开 direct-tcpip：拿不到 channel-open 应答，直接卡死
    let proxy_port = spawn_production_dynamic_proxy(handle.clone()).await;
    tokio::time::sleep(Duration::from_millis(30)).await;
    let result = tokio::time::timeout(Duration::from_secs(3), socks5_connect_ipv4(proxy_port, http_addr)).await;
    let stalled = result.is_err();
    assert!(stalled, "失联共享连接上 direct-tcpip 应卡死（SOCKS5 超时），而 is_closed() 检测不到");
    assert!(!handle.is_closed(), "卡死期间 is_closed() 依旧为 false，判活门无法发现这条死连接");

    server_task.abort();
}

#[tokio::test]
async fn dynamic_e2e_http_ok_with_production_order() {
    let marker = "TERMIX-DYNAMIC-OK";
    let http_addr = start_http_target(marker).await;
    let delay = Arc::new(AsyncMutex::new(Duration::ZERO));
    let force_fail = Arc::new(AsyncMutex::new(false));
    let last_target = Arc::new(AsyncMutex::new(None));
    let (handle, server_task) = open_ssh_client(DirectTcpipServer { delay_before_accept: delay, force_fail, last_target: last_target.clone() }).await;
    let handle = Arc::new(handle);
    let proxy_port = spawn_production_dynamic_proxy(handle.clone()).await;
    tokio::time::sleep(Duration::from_millis(30)).await;

    let stream = socks5_connect_ipv4(proxy_port, http_addr).await.expect("socks connect");
    let body = http_get_over(stream).await;
    let target = last_target.lock().await.clone();
    assert!(body.contains(marker), "生产顺序(先 direct-tcpip 再 socks 成功)应拿到 HTTP, body={} target={:?}", body, target);
    assert_eq!(target, Some((http_addr.ip().to_string(), http_addr.port() as u32)));

    server_task.abort();
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "EN").await;
}

#[tokio::test]
async fn dynamic_early_socks_success_stalls_when_direct_tcpip_is_slow() {
    let marker = "TERMIX-DYNAMIC-OK";
    let http_addr = start_http_target(marker).await;
    let delay = Arc::new(AsyncMutex::new(Duration::from_secs(2)));
    let force_fail = Arc::new(AsyncMutex::new(false));
    let last_target = Arc::new(AsyncMutex::new(None));
    let (handle, server_task) = open_ssh_client(DirectTcpipServer { delay_before_accept: delay, force_fail, last_target }).await;
    let handle = Arc::new(handle);
    let proxy_port = spawn_buggy_early_success_proxy(handle.clone()).await;
    tokio::time::sleep(Duration::from_millis(30)).await;

    let started = Instant::now();
    let stream = socks5_connect_ipv4(proxy_port, http_addr).await.expect("socks 会立刻成功(错误时序)");
    let socks_elapsed = started.elapsed();
    assert!(socks_elapsed < Duration::from_millis(500), "错误时序下 SOCKS 成功应答过早: {:?}", socks_elapsed);

    let body = http_get_over(stream).await;
    assert!(
        !body.contains(marker) || started.elapsed() >= Duration::from_secs(2),
        "先回 SOCKS 成功再慢开隧道时，客户端会在隧道未就绪时发请求，表现为超时/空响应(对应用户 curl 卡在 TLS)。body={} elapsed={:?}",
        body,
        started.elapsed()
    );

    server_task.abort();
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "EN").await;
}

#[tokio::test]
async fn dynamic_production_order_waits_for_direct_tcpip_before_socks_success() {
    let marker = "TERMIX-DYNAMIC-OK";
    let http_addr = start_http_target(marker).await;
    let delay = Arc::new(AsyncMutex::new(Duration::from_millis(800)));
    let force_fail = Arc::new(AsyncMutex::new(false));
    let last_target = Arc::new(AsyncMutex::new(None));
    let (handle, server_task) = open_ssh_client(DirectTcpipServer { delay_before_accept: delay, force_fail, last_target }).await;
    let handle = Arc::new(handle);
    let proxy_port = spawn_production_dynamic_proxy(handle.clone()).await;
    tokio::time::sleep(Duration::from_millis(30)).await;

    let started = Instant::now();
    let stream = socks5_connect_ipv4(proxy_port, http_addr).await.expect("socks");
    let socks_elapsed = started.elapsed();
    assert!(socks_elapsed >= Duration::from_millis(700), "生产顺序应等 direct-tcpip 完成才回 SOCKS 成功, elapsed={:?}", socks_elapsed);
    let body = http_get_over(stream).await;
    assert!(body.contains(marker), "隧道就绪后再回成功，HTTP 应立即通, body={}", body);

    server_task.abort();
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "EN").await;
}

#[tokio::test]
async fn dynamic_direct_tcpip_failure_returns_socks_failure() {
    let http_addr = start_http_target("x").await;
    let delay = Arc::new(AsyncMutex::new(Duration::ZERO));
    let force_fail = Arc::new(AsyncMutex::new(true));
    let last_target = Arc::new(AsyncMutex::new(None));
    let (handle, server_task) = open_ssh_client(DirectTcpipServer { delay_before_accept: delay, force_fail, last_target }).await;
    let handle = Arc::new(handle);
    let proxy_port = spawn_production_dynamic_proxy(handle.clone()).await;
    tokio::time::sleep(Duration::from_millis(30)).await;

    let err = socks5_connect_ipv4(proxy_port, http_addr).await.expect_err("direct-tcpip 失败时应收到 socks 失败而不是挂死");
    let msg = err.to_string();
    assert!(msg.contains("socks reply status"), "应是 SOCKS 失败码, err={}", msg);

    server_task.abort();
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "EN").await;
}
