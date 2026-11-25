use std::sync::Arc;
use parking_lot::Mutex;
use std::future::Future;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::task::AbortHandle;
use std::thread::JoinHandle as OsJoinHandle;

const DEFAULT_JOIN_WAIT: Duration = Duration::from_secs(2);

struct Inner {
    async_tasks: Mutex<Vec<AbortHandle>>,
    os_threads: Mutex<Vec<OsJoinHandle<()>>>,
}

impl Drop for Inner {
    fn drop(&mut self) {
        for handle in self.async_tasks.lock().drain(..) {
            handle.abort();
        }
    }
}

#[derive(Clone)]
pub struct TaskGroup {
    inner: Arc<Inner>,
}

impl Default for TaskGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskGroup {
    pub fn new() -> Self {
        Self { inner: Arc::new(Inner { async_tasks: Mutex::new(Vec::new()), os_threads: Mutex::new(Vec::new()) }) }
    }

    pub fn global() -> &'static TaskGroup {
        static GLOBAL: OnceLock<TaskGroup> = OnceLock::new();
        GLOBAL.get_or_init(TaskGroup::new)
    }

    pub fn spawn<F>(&self, _name: &'static str, fut: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let handle = tokio::spawn(fut);
        self.inner.async_tasks.lock().push(handle.abort_handle());
    }

    pub fn spawn_blocking<F>(&self, _name: &'static str, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let handle = tokio::task::spawn_blocking(f);
        self.inner.async_tasks.lock().push(handle.abort_handle());
    }

    pub fn spawn_os<F>(&self, name: &str, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        match std::thread::Builder::new().name(name.to_string()).spawn(f) {
            Ok(handle) => self.adopt_os(handle),
            Err(error) => summer::tracing::warn!("[system] os thread `{name}` spawn failed: {error}"),
        }
    }

    pub fn spawn_os_required<F>(&self, name: &str, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let handle = std::thread::Builder::new().name(name.to_string()).spawn(f).unwrap_or_else(|error| panic!("failed to start {name} thread: {error}"));
        self.adopt_os(handle);
    }

    pub fn adopt_os(&self, handle: OsJoinHandle<()>) {
        self.inner.os_threads.lock().push(handle);
    }

    pub fn abort(&self) {
        for handle in self.inner.async_tasks.lock().drain(..) {
            handle.abort();
        }
    }

    pub async fn join_os(&self, wait: Duration) {
        let handles = std::mem::take(&mut *self.inner.os_threads.lock());
        join_os_handles(handles, wait).await;
    }

    pub async fn shutdown(&self) {
        self.shutdown_within(DEFAULT_JOIN_WAIT).await;
    }

    pub async fn shutdown_within(&self, wait: Duration) {
        self.abort();
        tokio::task::yield_now().await;
        self.join_os(wait).await;
    }
}

pub async fn join_os_handles(handles: Vec<OsJoinHandle<()>>, wait: Duration) {
    if handles.is_empty() {
        return;
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    let spawned = std::thread::Builder::new().name("join-os".into()).spawn(move || {
        for handle in handles {
            let _ = handle.join();
        }
        let _ = tx.send(());
    });
    if spawned.is_err() {
        return;
    }
    let _ = tokio::time::timeout(wait, rx).await;
}
