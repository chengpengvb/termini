use std::sync::Arc;
use dashmap::DashMap;
use std::time::Duration;
use russh::{client, server};
use tokio::task::JoinHandle;
use russh::ChannelOpenFailure;
use parking_lot::Mutex as SyncMutex;
use russh::client::ChannelOpenHandle;
use tokio::sync::Mutex as AsyncMutex;
use russh::keys::{Algorithm, PrivateKey};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::module::ssh_forward::pump::forward_pump;
use crate::module::ssh_forward::pump::forward_sink::ForwardSink;
use russh::server::{Auth, Msg as ServerMsg, Server as _, Session};

fn lookup_forward(registry: &DashMap<(String, u16), ForwardSink>, address: &str, port: u16) -> Option<ForwardSink> {
    registry.get(&(address.to_string(), port)).map(|entry| entry.value().clone())
}

struct ClientHandler {
    registry: Arc<DashMap<(String, u16), ForwardSink>>,
    last_lookup: Arc<AsyncMutex<Option<(String, u16, bool)>>>,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(&mut self, _key: &russh::keys::PublicKey) -> Result<bool, Self::Error> {
        Ok(true)
    }

    async fn server_channel_open_forwarded_tcpip(
        &mut self,
        channel: russh::Channel<client::Msg>,
        connected_address: &str,
        connected_port: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: ChannelOpenHandle,
        _session: &mut client::Session,
    ) -> Result<(), Self::Error> {
        let hit = lookup_forward(&self.registry, connected_address, connected_port as u16);
        {
            let mut guard = self.last_lookup.lock().await;
            *guard = Some((connected_address.to_string(), connected_port as u16, hit.is_some()));
        }
        let Some(sink) = hit else {
            reply.reject(ChannelOpenFailure::AdministrativelyProhibited).await;
            return Ok(());
        };
        reply.accept().await;
        let target = format!("{}:{}", sink.target_host, sink.target_port);
        let conns = sink.conns.clone();
        let task = tokio::spawn(async move {
            let tcp = match TcpStream::connect(&target).await {
                Ok(tcp) => tcp,
                Err(_) => return,
            };
            forward_pump::pump(channel, tcp).await;
        });
        conns.lock().push(task);
        Ok(())
    }
}

#[derive(Clone)]
struct ForwardSshServer;

struct ForwardSshHandler;

impl server::Server for ForwardSshServer {
    type Handler = ForwardSshHandler;

    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self::Handler {
        ForwardSshHandler
    }
}

impl server::Handler for ForwardSshHandler {
    type Error = russh::Error;

    async fn auth_password(&mut self, _user: &str, _password: &str) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn auth_publickey(&mut self, _user: &str, _key: &russh::keys::PublicKey) -> Result<Auth, Self::Error> {
        Ok(Auth::Accept)
    }

    async fn tcpip_forward(&mut self, address: &str, port: &mut u32, session: &mut Session) -> Result<bool, Self::Error> {
        let bind_host = if address.is_empty() { "127.0.0.1" } else { address };
        let listener = TcpListener::bind((bind_host, *port as u16)).await.map_err(|_| russh::Error::WrongChannel)?;
        let bound_port = listener.local_addr().map_err(|_| russh::Error::WrongChannel)?.port();
        *port = bound_port as u32;
        let handle = session.handle();
        let report = address.to_string();
        tokio::spawn(async move {
            loop {
                let (mut tcp, peer) = match listener.accept().await {
                    Ok(value) => value,
                    Err(_) => break,
                };
                let handle = handle.clone();
                let report = report.clone();
                tokio::spawn(async move {
                    let channel = match handle.channel_open_forwarded_tcpip(report, bound_port as u32, peer.ip().to_string(), peer.port() as u32).await {
                        Ok(channel) => channel,
                        Err(_) => return,
                    };
                    let mut stream = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut stream, &mut tcp).await;
                });
            }
        });
        Ok(true)
    }

    async fn channel_open_session(&mut self, _channel: russh::Channel<ServerMsg>, reply: server::ChannelOpenHandle, _session: &mut Session) -> Result<(), Self::Error> {
        reply.accept().await;
        Ok(())
    }
}

async fn start_http_target(body: &'static str) -> (std::net::SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind http target");
    let addr = listener.local_addr().expect("http local addr");
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else { break };
            let mut buf = [0u8; 2048];
            let _ = sock.read(&mut buf).await;
            let response = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body);
            let _ = sock.write_all(response.as_bytes()).await;
        }
    });
    (addr, handle)
}

async fn open_ssh_session(registry: Arc<DashMap<(String, u16), ForwardSink>>, last_lookup: Arc<AsyncMutex<Option<(String, u16, bool)>>>) -> (client::Handle<ClientHandler>, JoinHandle<()>, u16) {
    let marker_port = {
        let probe = TcpListener::bind("127.0.0.1:0").await.expect("probe");
        probe.local_addr().expect("addr").port()
    };
    let key = PrivateKey::random(&mut rand10::rng(), Algorithm::Ed25519).expect("ssh key");
    let config = Arc::new(server::Config { keys: vec![key], ..Default::default() });
    let mut server = ForwardSshServer;
    let ssh_listener = TcpListener::bind("127.0.0.1:0").await.expect("bind ssh");
    let ssh_addr = ssh_listener.local_addr().expect("ssh addr");
    let server_task = tokio::spawn(async move {
        let run = server.run_on_socket(config, &ssh_listener);
        run.await.ok();
    });
    let handler = ClientHandler { registry, last_lookup };
    let mut handle = client::connect(Arc::new(client::Config::default()), ssh_addr, handler).await.expect("ssh connect");
    let auth = handle.authenticate_password("test", "test").await.expect("auth");
    assert!(auth.success(), "password auth should succeed");
    (handle, server_task, marker_port)
}

#[tokio::test]
async fn russh_tcpip_forward_returns_zero_for_nonzero_request_port() {
    let registry = Arc::new(DashMap::new());
    let last_lookup = Arc::new(AsyncMutex::new(None));
    let (handle, server_task, request_port) = open_ssh_session(registry, last_lookup).await;
    let returned = handle.tcpip_forward("127.0.0.1".to_string(), request_port as u32).await.expect("tcpip_forward");
    assert_eq!(returned, 0, "russh: 非 0 端口请求成功时返回值是 0，不是请求端口 {}", request_port);
    server_task.abort();
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "EN").await;
}

#[tokio::test]
async fn production_register_key_uses_returned_zero_and_misses_lookup() {
    let marker = "TERMIX-FORWARD-OK";
    let (http_addr, http_task) = start_http_target(marker).await;
    let registry: Arc<DashMap<(String, u16), ForwardSink>> = Arc::new(DashMap::new());
    let last_lookup = Arc::new(AsyncMutex::new(None));
    let (handle, server_task, request_port) = open_ssh_session(registry.clone(), last_lookup.clone()).await;

    let returned = handle.tcpip_forward("127.0.0.1".to_string(), request_port as u32).await.expect("tcpip_forward") as u16;
    assert_eq!(returned, 0);

    registry.insert(("127.0.0.1".to_string(), returned), ForwardSink { target_host: http_addr.ip().to_string(), target_port: http_addr.port(), conns: Arc::new(SyncMutex::new(Vec::new())) });

    tokio::time::sleep(Duration::from_millis(50)).await;
    let mut remote = TcpStream::connect(("127.0.0.1", request_port)).await.expect("connect remote listen");
    remote.write_all(b"GET / HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n").await.expect("write http");
    let mut buf = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(2), remote.read_to_end(&mut buf)).await;
    let text = String::from_utf8_lossy(&buf).to_string();
    let lookup = last_lookup.lock().await.clone().expect("应触发 forwarded-tcpip");

    assert_eq!(lookup.0, "127.0.0.1");
    assert_eq!(lookup.1, request_port, "sshd 回传的 connected_port 是真实监听端口");
    assert!(!lookup.2, "生产写法 register(port=tcpip_forward返回值0) 会导致 lookup(真实端口) 落空");
    assert!(!text.contains(marker), "lookup 落空后远端拿不到本机 HTTP, body={}", text);

    http_task.abort();
    server_task.abort();
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "EN").await;
}

#[tokio::test]
async fn remote_forward_ok_when_register_uses_requested_port_not_returned_zero() {
    let marker = "TERMIX-FORWARD-OK";
    let (http_addr, http_task) = start_http_target(marker).await;
    let registry: Arc<DashMap<(String, u16), ForwardSink>> = Arc::new(DashMap::new());
    let last_lookup = Arc::new(AsyncMutex::new(None));
    let (handle, server_task, request_port) = open_ssh_session(registry.clone(), last_lookup.clone()).await;

    let returned = handle.tcpip_forward("127.0.0.1".to_string(), request_port as u32).await.expect("tcpip_forward") as u16;
    assert_eq!(returned, 0);

    registry.insert(("127.0.0.1".to_string(), request_port), ForwardSink { target_host: http_addr.ip().to_string(), target_port: http_addr.port(), conns: Arc::new(SyncMutex::new(Vec::new())) });

    tokio::time::sleep(Duration::from_millis(50)).await;
    let mut remote = TcpStream::connect(("127.0.0.1", request_port)).await.expect("connect remote listen");
    remote.write_all(b"GET / HTTP/1.1\r\nHost: test\r\nConnection: close\r\n\r\n").await.expect("write http");
    let mut buf = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(2), remote.read_to_end(&mut buf)).await;
    let text = String::from_utf8_lossy(&buf).to_string();
    let lookup = last_lookup.lock().await.clone().expect("应触发 forwarded-tcpip");

    assert!(lookup.2, "用请求端口注册后 lookup 应命中: {:?}", lookup);
    assert!(text.contains(marker), "修 key 后应拿到本机 HTTP, body={}", text);

    http_task.abort();
    server_task.abort();
    let _ = handle.disconnect(russh::Disconnect::ByApplication, "", "EN").await;
}
