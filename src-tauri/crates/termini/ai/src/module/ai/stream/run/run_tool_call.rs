use rig::message::ToolCall;
use std::collections::HashSet;
use crate::util::id_factory::uuid;
use summer::tracing::{debug, warn};
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::stream::stream_runner::{AiStreamRunner, OpenSegment, TOOL_CALL_CANCELLED_RESULT};

impl AiStreamRunner {
    pub(in crate::module::ai::stream) async fn handle_stream_tool_call(
        &self,
        tool_call: &ToolCall,
        context: &AiRunContext,
        internal_call_id: String,
        tool_item_id: &mut Option<String>,
        text_segment: &mut Option<OpenSegment>,
        reasoning_segment: &mut Option<OpenSegment>,
    ) {
        self.finish_open_segments(context, text_segment, reasoning_segment).await;
        let id = self.open_tool_item_id(context, tool_item_id, &internal_call_id);
        self.handle_tool_cancel(context, internal_call_id, tool_call, id).await;
    }

    pub(in crate::module::ai::stream) async fn handle_stream_tool_result(
        &self,
        context: &AiRunContext,
        internal_call_id: String,
        tool_item_id: &mut Option<String>,
        tool_result: &rig::message::ToolResult,
        text_segment: &mut Option<OpenSegment>,
        reasoning_segment: &mut Option<OpenSegment>,
    ) {
        self.finish_open_segments(context, text_segment, reasoning_segment).await;
        self.handle_tool_result(context, &internal_call_id, tool_result).await;
        self.handle_tool_cancel_remove(context, internal_call_id.to_string()).await;
        self.chat_session_cache_manager.remove_tool_item_id(&context.req_id, &internal_call_id);
        *tool_item_id = None;
    }

    pub(in crate::module::ai::stream) fn finish_active_tool_calls(&self, context: &AiRunContext, active_call_ids: &mut HashSet<String>) {
        if context.cancel_token.is_cancelled() {
            active_call_ids.clear();
            return;
        }
        if active_call_ids.is_empty() {
            return;
        }
        warn!("[ai] 流结束仍有未完成的工具调用，补推完成事件 run_id={} pending={}", context.run_id, active_call_ids.len());
        for call_id in active_call_ids.drain() {
            self.tool_call_done_push.push_tool_call_done_event(context, &call_id, TOOL_CALL_CANCELLED_RESULT.to_string());
        }
    }

    fn open_tool_item_id(&self, context: &AiRunContext, tool_item_id: &mut Option<String>, internal_call_id: &str) -> String {
        let id = match self.chat_session_cache_manager.ensure_tool_item_id(&context.req_id, internal_call_id) {
            Ok(id) => id,
            Err(error) => {
                warn!("[ai] 登记 tool item id 失败 req_id={} call_id={} error={}", context.req_id, internal_call_id, error);
                uuid()
            }
        };
        *tool_item_id = Some(id.clone());
        id
    }

    async fn handle_tool_cancel(&self, context: &AiRunContext, internal_call_id: String, tool_call: &ToolCall, id: String) {
        if let Err(error) = self.chat_session_cache_manager.insert_tool_cancel(&context.req_id, context.run_id.clone(), internal_call_id.clone()) {
            warn!("[ai] 插入 tool cancel 失败 req_id={} run_id={} call_id={} error={}", context.req_id, context.run_id, internal_call_id, error);
        }
        self.handle_tool_call(context, &internal_call_id, tool_call, id).await;
    }

    async fn handle_tool_cancel_remove(&self, context: &AiRunContext, internal_call_id: String) {
        if let Err(error) = self.chat_session_cache_manager.remove_tool_cancel(&context.req_id, &context.run_id, &internal_call_id) {
            debug!("[ai] 移除 tool cancel 未找到记录 req_id={} run_id={} call_id={} error={}", context.req_id, context.run_id, internal_call_id, error);
        }
    }
}
