use std::error::Error;
use std::future::Future;
use tokio::sync::Notify;
use std::sync::atomic::AtomicBool;
use std::fmt::{Display, Formatter};
use std::sync::atomic::Ordering::SeqCst;

pub struct CancelNotify<'a> {
    pub notify: &'a Notify,
    cancelled: &'a AtomicBool,
}

impl<'a> CancelNotify<'a> {
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(SeqCst)
    }
}

pub async fn await_with_cancel<T, F, Fut>(cancel: &CancelNotify<'_>, work: F) -> anyhow::Result<T>
where
    T: Send,
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<T>> + Send,
{
    if cancel.is_cancelled() {
        return Err(cancelled());
    }
    let cancel_notified = cancel.notify.notified();
    tokio::pin!(cancel_notified);
    cancel_notified.as_mut().enable();
    tokio::select! {
        biased;
        _ = &mut cancel_notified => Err(cancelled()),
        result = work() => result,
    }
}

pub struct CancelSession {
    pub cancel: Notify,
    cancelled: AtomicBool,
}

impl CancelSession {
    pub fn new() -> Self {
        Self { cancel: Notify::new(), cancelled: AtomicBool::new(false) }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, SeqCst);
        self.cancel.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(SeqCst)
    }

    pub fn cancel_notify(&self) -> CancelNotify<'_> {
        CancelNotify { notify: &self.cancel, cancelled: &self.cancelled }
    }
}

#[derive(Debug)]
pub struct Cancelled;

impl Display for Cancelled {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "cancelled")
    }
}

impl Error for Cancelled {}

pub fn cancelled() -> anyhow::Error {
    Cancelled.into()
}

pub fn is_cancelled(err: &anyhow::Error) -> bool {
    err.downcast_ref::<Cancelled>().is_some()
}
