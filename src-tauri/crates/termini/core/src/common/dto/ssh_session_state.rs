use std::sync::Arc;
use dashmap::DashMap;
use russh::client::Handle;
use russh::client::Handler;
use std::collections::VecDeque;
use parking_lot::Mutex as SyncMutex;
use tokio::sync::Mutex as AsyncMutex;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::common::dto::ssh_auth_input::SshAuthAnswer;
use crate::common::dto::ssh_session_key::SshSessionKey;
use crate::model::resp::event::resp_event_ssh_auth::SshOpenCallback;

pub struct SshAuthAnswerEntry {
    pub request_id: String,
    pub answer: SshAuthAnswer,
}

pub struct SshAuthInbox {
    pub cancelled: AtomicBool,
    pub notify: tokio::sync::Notify,
    pub cancel_notify: tokio::sync::Notify,
    pub queue: AsyncMutex<VecDeque<SshAuthAnswerEntry>>,
}

impl SshAuthInbox {
    pub fn new() -> Self {
        Self { notify: tokio::sync::Notify::new(), queue: AsyncMutex::new(VecDeque::new()), cancel_notify: tokio::sync::Notify::new(), cancelled: AtomicBool::new(false) }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
        self.cancel_notify.notify_waiters();
        self.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

pub struct SshChannel<H: Handler> {
    pub handle: Arc<Handle<H>>,
    pub key: Option<SshSessionKey>,
    pub callback: SshOpenCallback,
}

pub struct SshSharedConn<H: Handler> {
    pub connect_lock: AsyncMutex<()>,
    pub handle: SyncMutex<Option<Arc<Handle<H>>>>,
}

impl<H: Handler> SshSharedConn<H> {
    pub fn new() -> Self {
        Self { connect_lock: AsyncMutex::new(()), handle: SyncMutex::new(None) }
    }
}

pub struct SshSessionFactoryState<H: Handler> {
    pub channels: DashMap<String, SshChannel<H>>,
    pub auth_inboxes: DashMap<String, Arc<SshAuthInbox>>,
    pub shared: DashMap<SshSessionKey, Arc<SshSharedConn<H>>>,
    pub auth_requests: Arc<DashMap<String, Arc<SshAuthInbox>>>,
}

impl<H: Handler> SshSessionFactoryState<H> {
    pub fn new() -> Self {
        Self { channels: DashMap::new(), auth_inboxes: DashMap::new(), shared: DashMap::new(), auth_requests: Arc::new(DashMap::new()) }
    }
}
