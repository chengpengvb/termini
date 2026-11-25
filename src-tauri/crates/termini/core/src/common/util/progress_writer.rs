use std::pin::Pin;
use std::sync::Arc;
use parking_lot::Mutex;
use std::time::Instant;
use tokio::io::AsyncWrite;
use std::task::{Context, Poll};
use crate::common::util::task_hub::TaskGroup;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::model::resp::resp_sftp::{RespSftpTfProgress, RespSftpTransferProgressHandler};

const PROGRESS_REPORT_INTERVAL_MS: u64 = 500;
const PROGRESS_REPORT_MIN_BYTES: u64 = 256 * 1024;

struct ProgressFlush {
    flushing: AtomicBool,
    pending: Mutex<Option<RespSftpTfProgress>>,
}

pub struct ProgressWriter {
    done: u64,
    file: String,
    total: Option<u64>,
    last_report_bytes: u64,
    last_report_time: Instant,
    flush: Arc<ProgressFlush>,
    handler: RespSftpTransferProgressHandler,
    inner: Pin<Box<dyn AsyncWrite + Send>>,
}

impl ProgressWriter {
    pub fn new(inner: Pin<Box<dyn AsyncWrite + Send>>, file: String, total: Option<u64>, start_offset: u64, handler: RespSftpTransferProgressHandler) -> Self {
        Self {
            inner,
            file,
            total,
            handler,
            done: start_offset,
            last_report_bytes: start_offset,
            last_report_time: Instant::now(),
            flush: Arc::new(ProgressFlush { pending: Mutex::new(None), flushing: AtomicBool::new(false) }),
        }
    }

    fn record_written(&mut self, written: u64) {
        self.done += written;
        let now = Instant::now();
        let elapsed_ms = now.duration_since(self.last_report_time).as_millis() as u64;
        let bytes_since = self.done - self.last_report_bytes;
        if elapsed_ms < PROGRESS_REPORT_INTERVAL_MS && bytes_since < PROGRESS_REPORT_MIN_BYTES {
            return;
        }
        let speed = if elapsed_ms > 0 { bytes_since * 1000 / elapsed_ms } else { 0 };
        let progress = RespSftpTfProgress { file: self.file.clone(), transfer_done: self.done, transfer_total: self.total.unwrap_or(0), speed, ..Default::default() };
        *self.flush.pending.lock() = Some(progress);
        self.last_report_time = now;
        self.last_report_bytes = self.done;
        self.kick_flush();
    }

    fn kick_flush(&self) {
        if self.flush.flushing.swap(true, Ordering::AcqRel) {
            return;
        }
        let flush = Arc::clone(&self.flush);
        let handler = self.handler.clone();
        TaskGroup::global().spawn("sftp-progress", async move {
            loop {
                let progress = flush.pending.lock().take();
                match progress {
                    Some(progress) => {
                        let _ = handler(progress).await;
                    }
                    None => {
                        flush.flushing.store(false, Ordering::Release);
                        if flush.pending.lock().is_none() {
                            break;
                        }
                        if flush.flushing.swap(true, Ordering::AcqRel) {
                            break;
                        }
                    }
                }
            }
        });
    }
}

impl AsyncWrite for ProgressWriter {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        let me = self.get_mut();
        let result = me.inner.as_mut().poll_write(cx, buf);
        if let Poll::Ready(Ok(n)) = result {
            me.record_written(n as u64);
        }
        result
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.get_mut().inner.as_mut().poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        self.get_mut().inner.as_mut().poll_shutdown(cx)
    }
}
