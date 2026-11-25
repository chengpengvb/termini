use crate::module::ai::dto::AiRunContext;
use rig::message::{ToolResult, ToolResultContent};
use crate::module::ai::stream::stream_runner::AiStreamRunner;

impl AiStreamRunner {
    pub(in crate::module::ai::stream) async fn handle_tool_result(&self, context: &AiRunContext, internal_call_id: &str, tool_result: &ToolResult) {
        let result = Self::tool_result_text(&tool_result.content);
        self.tool_call_done_push.push_tool_call_done_event(context, internal_call_id, result);
    }

    fn tool_result_text(content: &[ToolResultContent]) -> String {
        content.iter().filter_map(|item| item.as_text()).collect::<Vec<_>>().join("\n")
    }
}
