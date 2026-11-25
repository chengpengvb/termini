use std::sync::Arc;
use parking_lot::Mutex;

#[derive(Clone)]
pub struct ClipboardImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub png: Vec<u8>,
}

#[derive(Clone)]
pub struct ClipboardSnapshot {
    pub text: String,
    pub html: String,
    pub files: Vec<String>,
    pub image: Option<ClipboardImage>,
}

#[derive(Clone)]
pub enum ClipboardEvent {
    Unavailable,
    Snapshot(Arc<ClipboardSnapshot>),
}

pub type ClipboardListener = Arc<dyn Fn(ClipboardEvent) + Send + Sync>;

pub struct ClipboardSubscription {
    listeners: Arc<Mutex<Vec<(u64, ClipboardListener)>>>,
    id: u64,
}

impl ClipboardSubscription {
    pub fn new(listeners: Arc<Mutex<Vec<(u64, ClipboardListener)>>>, id: u64) -> Self {
        Self { listeners, id }
    }
}

impl Drop for ClipboardSubscription {
    fn drop(&mut self) {
        self.listeners.lock().retain(|(id, _)| *id != self.id);
    }
}
