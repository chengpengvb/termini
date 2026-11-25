use std::sync::Arc;
use anyhow::Context;
use summer::async_trait;
use tokio::sync::Notify;
use std::net::SocketAddr;
use summer::plugin::Plugin;
use summer::event::EventPublisher;
use summer::tracing::{info, warn};
use summer::app::{App, AppBuilder};
use summer_web::config::ServerConfig;
use tokio::time::{timeout, Duration};
use tokio::net::{TcpListener, TcpStream};
use summer_web::{assemble_router, axum, finalize_router, ServerProtocol, ServerStartedEvent};

const PROBE_TIMEOUT: Duration = Duration::from_secs(1);

static REBIND: Notify = Notify::const_new();

pub struct WebServerPlugin;

#[async_trait]
impl Plugin for WebServerPlugin {
    async fn build(&self, app: &mut AppBuilder) {
        let config = assemble_router(app).await;
        app.add_scheduler(move |app: Arc<App>| Box::new(WebServer { app, config }.run()));
    }
}

#[cfg(mobile)]
pub fn request_rebind() {
    REBIND.notify_one();
}

struct WebServer {
    app: Arc<App>,
    config: ServerConfig,
}

impl WebServer {
    async fn run(self) -> summer::error::Result<String> {
        let router = finalize_router(&self.app, &self.config.global_prefix);
        let addr = SocketAddr::from((self.config.binding, self.config.port));
        let mut started = false;
        loop {
            let listener = TcpListener::bind(addr).await.with_context(|| format!("bind tcp listener failed:{addr}"))?;
            info!("[web] bind tcp listener: {addr}");
            if !started {
                started = true;
                self.app.publish(ServerStartedEvent { addr, protocol: ServerProtocol::Http }).await?;
            }
            let serve = axum::serve(listener, router.clone().into_make_service());
            tokio::select! {
                result = serve => {
                    result.context("start axum server failed")?;
                    return Ok("axum schedule finished".to_string());
                }
                _ = Self::wait_listener_dead(addr) => {
                    warn!("[web] 监听 {addr} 在挂起后已失效, 重新绑定");
                }
            }
        }
    }

    async fn wait_listener_dead(addr: SocketAddr) {
        loop {
            REBIND.notified().await;
            if !Self::listener_alive(addr).await {
                return;
            }
        }
    }

    async fn listener_alive(addr: SocketAddr) -> bool {
        matches!(timeout(PROBE_TIMEOUT, TcpStream::connect(addr)).await, Ok(Ok(_)))
    }
}
