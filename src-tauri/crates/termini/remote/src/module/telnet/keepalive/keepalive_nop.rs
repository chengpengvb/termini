use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::io::AsyncWriteExt;
use summer::tracing::{info, warn};
use parking_lot::Mutex as SyncMutex;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::sync::{oneshot, Mutex as AsyncMutex};
use tokio::time::{interval, Duration, MissedTickBehavior};
use crate::module::telnet::core::command::{TELNET_IAC, TELNET_NOP};

const KEEPALIVE_NOP_BYTES: [u8; 2] = [TELNET_IAC, TELNET_NOP];

pub struct KeepaliveNop {
    keepalive_interval_seconds: u64,
    writer: Arc<AsyncMutex<OwnedWriteHalf>>,
    job_handle: SyncMutex<Option<JoinHandle<()>>>,
    close_tx: SyncMutex<Option<oneshot::Sender<()>>>,
}

impl KeepaliveNop {
    pub fn new(writer: Arc<AsyncMutex<OwnedWriteHalf>>, keepalive_interval_seconds: u64) -> Self {
        Self { writer, keepalive_interval_seconds, close_tx: SyncMutex::new(None), job_handle: SyncMutex::new(None) }
    }

    async fn run_job(mut close_rx: oneshot::Receiver<()>, writer: Arc<AsyncMutex<OwnedWriteHalf>>, keepalive_interval_seconds: u64) {
        let mut ticker = interval(Duration::from_secs(keepalive_interval_seconds));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let _ = ticker.tick().await; // skip immediate first tick
        info!("[telnet] keepalive interval: {}s", keepalive_interval_seconds);
        loop {
            tokio::select! {
                _ = &mut close_rx => {
                    break;
                }
                _ = ticker.tick() => {
                    let write_result = tokio::select! {
                        _ = &mut close_rx => {
                            break;
                        }
                        result = async {
                            let mut guard = writer.lock().await;
                            guard.write_all(&KEEPALIVE_NOP_BYTES).await
                        } => result,
                    };
                    if let Err(error) = write_result {
                        warn!("[telnet] write NOP failed: {}", error);
                        break;
                    };
                }
            }
        }
    }

    pub fn start_job(&self) {
        if self.keepalive_interval_seconds == 0 {
            return;
        }
        if self.job_handle.lock().is_some() {
            return;
        }
        let (close_tx, close_rx) = oneshot::channel::<()>();
        *self.close_tx.lock() = Some(close_tx);

        let writer = Arc::clone(&self.writer);
        let keepalive_interval_seconds = self.keepalive_interval_seconds;
        let handle = tokio::spawn(async move {
            Self::run_job(close_rx, writer, keepalive_interval_seconds).await;
        });
        *self.job_handle.lock() = Some(handle);
    }

    pub async fn close(&self) {
        if let Some(tx) = self.close_tx.lock().take() {
            let _ = tx.send(());
        }
        let handle = self.job_handle.lock().take();
        if let Some(handle) = handle {
            let _ = handle.await;
        }
    }
}
