use std::pin::Pin;
use anyhow::Result;
use std::sync::Arc;
use std::future::Future;
use async_trait::async_trait;
use crate::common::context::context_base::WindowSize;

pub type EventHandler<E> = Arc<dyn Fn(E) -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

#[async_trait]
pub trait ClientTerminal: Send + Sync {
    type Event: Send + 'static;
    type Context: Send;

    async fn close(&self) -> Result<()>;
    async fn write(&self, data: String) -> Result<()>;
    async fn set_window_size(&self, size: WindowSize) -> Result<()>;
    async fn open(&self, context: Arc<Self::Context>, handler: EventHandler<Self::Event>) -> Result<()>;
}
