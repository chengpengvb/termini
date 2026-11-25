use regex::Regex;
use std::pin::Pin;
use std::sync::Arc;
use parking_lot::Mutex;
use std::future::Future;
use std::time::Duration;
use crate::util::time_util::now_millis;
use tokio::time::{sleep, Instant as TokioInstant};
use crate::module::common::dto::shell_chunk_callback::ShellChunkCallback;
use crate::module::ai::tools::common::shell::local_shell_registry::ShellProcessEntry;

pub const WAKE_HIT_LINES_MAX: usize = 20;
pub const WAKE_POLL_INTERVAL: Duration = Duration::from_millis(500);

pub async fn sleep_or_wake(entry: &ShellProcessEntry, wait: Duration, patterns: &[String], from: usize) -> Option<String> {
    if patterns.is_empty() {
        sleep(wait).await;
        return None;
    }
    let deadline = TokioInstant::now() + wait;
    loop {
        let now = TokioInstant::now();
        if now >= deadline {
            return None;
        }
        let step = WAKE_POLL_INTERVAL.min(deadline - now);
        sleep(step).await;
        if let Some(hit) = match_added(entry, patterns, from) {
            return Some(hit);
        }
    }
}

pub async fn wait_regex_hit(entry: &ShellProcessEntry, pattern: Option<&Regex>, since: &mut usize) -> String {
    let Some(regex) = pattern else {
        return std::future::pending().await;
    };
    loop {
        sleep(WAKE_POLL_INTERVAL).await;
        let (next, hit) = match_regex_lines(entry, regex, *since);
        *since = next;
        if let Some(lines) = hit {
            return lines;
        }
    }
}

pub fn match_regex_text(regex: &Regex, text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().filter(|line| regex.is_match(line)).take(WAKE_HIT_LINES_MAX).collect();
    if lines.is_empty() {
        return None;
    }
    Some(lines.join("\n"))
}

fn match_regex_lines(entry: &ShellProcessEntry, regex: &Regex, from: usize) -> (usize, Option<String>) {
    let buffer = entry.chunk_buffer.lock();
    if buffer.len() <= from {
        return (from, None);
    }
    let mut start = from.min(buffer.len());
    while start < buffer.len() && !buffer.is_char_boundary(start) {
        start += 1;
    }
    let Some(last_newline) = buffer[start..].rfind('\n') else {
        return (from, None);
    };
    let end = start + last_newline + 1;
    (end, match_regex_text(regex, &buffer[start..end]))
}

fn match_added(entry: &ShellProcessEntry, patterns: &[String], from: usize) -> Option<String> {
    let buffer = entry.chunk_buffer.lock();
    if buffer.len() <= from {
        return None;
    }
    let mut start = from.min(buffer.len());
    while start < buffer.len() && !buffer.is_char_boundary(start) {
        start += 1;
    }
    let added = buffer[start..].to_lowercase();
    patterns.iter().find(|pattern| added.contains(&pattern.to_lowercase())).cloned()
}

pub fn wrap_chunk_callback(on_chunk: ShellChunkCallback, chunk_buffer: Arc<Mutex<String>>, last_at_ms: Arc<Mutex<Option<i64>>>) -> ShellChunkCallback {
    Arc::new(move |bytes: Vec<u8>| {
        let on_chunk = on_chunk.clone();
        let chunk_buffer = chunk_buffer.clone();
        let last_at_ms = last_at_ms.clone();
        Box::pin(async move {
            let before = chunk_buffer.lock().len();
            on_chunk(bytes).await;
            if chunk_buffer.lock().len() != before {
                *last_at_ms.lock() = Some(now_millis());
            }
        }) as Pin<Box<dyn Future<Output = ()> + Send>>
    })
}
