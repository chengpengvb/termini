use std::sync::Arc;
use parking_lot::Mutex;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use std::sync::atomic::{AtomicU64, Ordering};
use crate::base::base_app::{AppContext, BaseApp};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
use crate::model::dto::clipboard::ClipboardImage;
use crate::model::dto::clipboard::{ClipboardEvent, ClipboardListener, ClipboardSnapshot, ClipboardSubscription};
#[cfg(not(any(target_os = "android", target_os = "ios")))]
const SELF_WRITE_IGNORE_WINDOW: std::time::Duration = std::time::Duration::from_millis(1500);

fn create_listeners() -> Arc<Mutex<Vec<(u64, ClipboardListener)>>> {
    Arc::new(Mutex::new(Vec::new()))
}

fn create_next_id() -> Arc<AtomicU64> {
    Arc::new(AtomicU64::new(1))
}

fn create_self_write() -> Arc<Mutex<Option<std::time::Instant>>> {
    Arc::new(Mutex::new(None))
}

fn create_latest() -> Arc<Mutex<Option<Arc<ClipboardSnapshot>>>> {
    Arc::new(Mutex::new(None))
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn create_watch_shutdown() -> Arc<Mutex<Option<clipboard_rs::WatcherShutdown>>> {
    Arc::new(Mutex::new(None))
}

#[base_app(sort = 900)]
#[derive(Clone, Service)]
pub struct ClipboardService {
    #[inject(func = create_next_id())]
    next_id: Arc<AtomicU64>,

    #[inject(func = create_self_write())]
    self_write: Arc<Mutex<Option<std::time::Instant>>>,

    #[inject(func = create_listeners())]
    listeners: Arc<Mutex<Vec<(u64, ClipboardListener)>>>,

    #[inject(func = create_latest())]
    latest_snapshot: Arc<Mutex<Option<Arc<ClipboardSnapshot>>>>,

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    #[inject(func = create_watch_shutdown())]
    watch_shutdown: Arc<Mutex<Option<clipboard_rs::WatcherShutdown>>>,
}

#[async_trait]
impl BaseApp for ClipboardService {
    fn name(&self) -> &'static str {
        "ClipboardService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.spawn_watch();
        Ok(())
    }

    async fn on_exit(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.stop_watch();
        Ok(())
    }
}

impl ClipboardService {
    pub fn subscribe(&self, listener: impl Fn(ClipboardEvent) + Send + Sync + 'static) -> ClipboardSubscription {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.listeners.lock().push((id, Arc::new(listener)));
        ClipboardSubscription::new(Arc::clone(&self.listeners), id)
    }

    pub fn latest(&self) -> Option<Arc<ClipboardSnapshot>> {
        self.latest_snapshot.lock().clone()
    }

    pub fn mark_self_write(&self) {
        *self.self_write.lock() = Some(std::time::Instant::now());
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    fn is_recent_self_write(&self) -> bool {
        match self.self_write.lock().as_ref() {
            Some(at) => at.elapsed() < SELF_WRITE_IGNORE_WINDOW,
            None => false,
        }
    }

    #[cfg(not(any(target_os = "android", target_os = "ios")))]
    fn emit(&self, event: ClipboardEvent) {
        if self.is_recent_self_write() {
            return;
        }
        if let ClipboardEvent::Snapshot(snapshot) = &event {
            *self.latest_snapshot.lock() = Some(Arc::clone(snapshot));
        }
        let listeners: Vec<ClipboardListener> = self.listeners.lock().iter().map(|(_, listener)| Arc::clone(listener)).collect();
        for listener in listeners {
            listener(event.clone());
        }
    }

    fn spawn_watch(&self) {
        #[cfg(any(target_os = "android", target_os = "ios"))]
        {
            let _ = self;
        }
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            let service = self.clone();
            let shutdown_slot = Arc::clone(&self.watch_shutdown);
            if let Ok(handle) = std::thread::Builder::new().name("clipboard-watch".into()).spawn(move || {
                run_watch(service, shutdown_slot);
            }) {
                crate::module::common::util::task_hub::TaskGroup::global().adopt_os(handle);
            }
        }
    }

    fn stop_watch(&self) {
        #[cfg(not(any(target_os = "android", target_os = "ios")))]
        {
            if let Some(shutdown) = self.watch_shutdown.lock().take() {
                shutdown.stop();
            }
        }
    }
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn run_watch(service: ClipboardService, shutdown_slot: Arc<Mutex<Option<clipboard_rs::WatcherShutdown>>>) {
    use summer::tracing::warn;
    use clipboard_rs::{ClipboardContext, ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext};

    struct Handler {
        ctx: ClipboardContext,
        service: ClipboardService,
    }

    impl ClipboardHandler for Handler {
        fn on_clipboard_change(&mut self) {
            self.service.emit(read_event(&self.ctx));
        }
    }

    let Ok(ctx) = ClipboardContext::new() else {
        warn!("[clipboard] clipboard watch ClipboardContext::new failed");
        return;
    };
    let Ok(mut watcher) = ClipboardWatcherContext::new() else {
        warn!("[clipboard] clipboard watch ClipboardWatcherContext::new failed");
        return;
    };
    service.emit(read_event(&ctx));
    watcher.add_handler(Handler { ctx, service });
    *shutdown_slot.lock() = Some(watcher.get_shutdown_channel());
    watcher.start_watch();
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn read_event(ctx: &clipboard_rs::ClipboardContext) -> ClipboardEvent {
    use clipboard_rs::{Clipboard, ContentFormat};
    let files = if ctx.has(ContentFormat::Files) {
        match ctx.get_files() {
            Ok(files) => files,
            Err(_) => {
                return ClipboardEvent::Unavailable;
            }
        }
    } else {
        Vec::new()
    };
    let text = ctx.get_text().unwrap_or_else(|_| String::new());
    let html = ctx.get_html().unwrap_or_else(|_| String::new());
    let image = read_image(ctx);
    ClipboardEvent::Snapshot(Arc::new(ClipboardSnapshot { text, html, files, image }))
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn read_image(ctx: &clipboard_rs::ClipboardContext) -> Option<ClipboardImage> {
    use clipboard_rs::Clipboard;
    use clipboard_rs::common::RustImage;
    let image = ctx.get_image().ok()?;
    if image.is_empty() {
        return None;
    }
    let (width, height) = image.get_size();
    let rgba = image.to_rgba8().ok()?.as_raw().to_vec();
    let png = image.to_png().ok()?.get_bytes().to_vec();
    Some(ClipboardImage { width, height, rgba, png })
}
