use std::pin::Pin;
use std::sync::Arc;
use std::future::Future;

pub type ShellChunkCallback = Arc<dyn Fn(Vec<u8>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;
