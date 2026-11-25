use std::sync::Arc;
use tokio::task::JoinHandle;
use parking_lot::Mutex as SyncMutex;

#[derive(Clone)]
pub struct ForwardSink {
    pub target_host: String,
    pub target_port: u16,
    pub conns: Arc<SyncMutex<Vec<JoinHandle<()>>>>,
}
