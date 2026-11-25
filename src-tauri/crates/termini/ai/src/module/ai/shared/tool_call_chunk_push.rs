use std::pin::Pin;
use std::sync::Arc;
use parking_lot::Mutex;
use std::future::Future;
use summer::tracing::warn;
use anyhow::{bail, Result};
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::module::ai::dto::AiRunContext;
use crate::service::service_agent::AgentService;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::util::ssh_shell::shell_text::decode_shell_bytes;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::module::common::dto::shell_chunk_callback::ShellChunkCallback;
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentRunBase, RespAgentToolCallChunk};

#[base_app(sort = 764)]
#[derive(Clone, Service)]
pub struct ToolCallChunkPush {
    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolCallChunkPush {
    fn name(&self) -> &'static str {
        "ToolCallChunkPush"
    }
}

impl ToolCallChunkPush {
    pub fn build_shell_chunk_callback(&self, context: &Arc<AiRunContext>, call_id: &str, tool_id: &str) -> (ShellChunkCallback, Arc<Mutex<String>>) {
        let pusher = self.clone();
        let context = Arc::clone(context);
        let call_id = call_id.to_string();
        let tool_id = tool_id.to_string();
        let buffer = Arc::new(Mutex::new(String::new()));
        let callback_buffer = Arc::clone(&buffer);
        let callback: ShellChunkCallback = Arc::new(move |bytes: Vec<u8>| {
            let pusher = pusher.clone();
            let call_id = call_id.clone();
            let tool_id = tool_id.clone();
            let context = Arc::clone(&context);
            let callback_buffer = Arc::clone(&callback_buffer);
            Box::pin(async move {
                let delta = decode_shell_bytes(&bytes);
                if !delta.is_empty() {
                    callback_buffer.lock().push_str(&delta);
                }
                if let Err(error) = pusher.push(&context, &call_id, &tool_id, delta) {
                    warn!("[ai] 推送实时回显失败 call_id={} tool_id={} error={:#}", call_id, tool_id, error);
                }
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        });
        (callback, buffer)
    }

    pub fn format_cancelled_output(buffer: &Mutex<String>) -> String {
        let output = buffer.lock().clone();
        if output.is_empty() {
            return "exit_code=cancelled".to_string();
        }
        format!("{}\nexit_code=cancelled", output)
    }

    pub fn push(&self, context: &AiRunContext, call_id: &str, tool_id: &str, delta: String) -> Result<()> {
        self.push_for_device(context, call_id, tool_id, None, delta)
    }

    pub fn push_for_device(&self, context: &AiRunContext, call_id: &str, tool_id: &str, device_id: Option<String>, delta: String) -> Result<()> {
        let Some(chat_session_cache_entry) = self.chat_session_cache_manager.get(&context.req_id) else {
            bail!("unknown req_id: {}", context.req_id);
        };
        let ai_chat_id = chat_session_cache_entry.context.ai_chat_id.clone();
        drop(chat_session_cache_entry);

        let pid = self.chat_session_cache_manager.get_tool_item_id(&context.req_id, call_id);
        let event = RespEventAgent::OnToolCallChunk(RespAgentToolCallChunk {
            delta,
            device_id,
            call_id: call_id.to_string(),
            tool_id: tool_id.to_string(),
            base: RespAgentBase::new(context.req_id.clone()),
            run: RespAgentRunBase { run_id: context.run_id.clone() },
        });
        self.agent_service.handler_agent_event(event, ai_chat_id.to_string(), None, pid, None);
        Ok(())
    }

    pub fn build_device_chunk_callback(&self, context: &Arc<AiRunContext>, call_id: &str, tool_id: &str, device_id: &str) -> ShellChunkCallback {
        let context = Arc::clone(context);
        let call_id = call_id.to_string();
        let tool_id = tool_id.to_string();
        let device_id = device_id.to_string();
        let pusher = self.clone();
        Arc::new(move |bytes: Vec<u8>| {
            let context = Arc::clone(&context);
            let call_id = call_id.clone();
            let tool_id = tool_id.clone();
            let device_id = device_id.clone();
            let pusher = pusher.clone();
            Box::pin(async move {
                let delta = decode_shell_bytes(&bytes);
                if delta.is_empty() {
                    return;
                }
                if let Err(error) = pusher.push_for_device(&context, &call_id, &tool_id, Some(device_id.clone()), delta) {
                    warn!("[ai] 推送设备回显失败 call_id={} device_id={} error={:#}", call_id, device_id, error);
                }
            }) as Pin<Box<dyn Future<Output = ()> + Send>>
        })
    }
}
