use anyhow::Result;
use std::sync::Arc;
use async_trait::async_trait;
use summer_web::socketioxide::extract::SocketRef;
use crate::model::req::req_ws_context::ReqSocketContext;

#[async_trait]
pub trait HandleBase: Send + Sync {
    fn get_name(&self) -> &'static str;

    fn contains_type(&self, _type_code: i16) -> bool;

    async fn handle_open(&self, _socket: SocketRef) -> Result<()> {
        Ok(())
    }

    async fn handle_close(&self, _socket: SocketRef) -> Result<()> {
        Ok(())
    }

    async fn handle_msg(&self, _socket: SocketRef, _req: Arc<ReqSocketContext>) -> Result<()>;
}
