use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::service::service_agent::AgentService;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::module::ai::tools::common::task::session::task_registry::TaskEntry;
use crate::model::resp::resp_agent::{RespAgentBase, RespAgentRunBase, RespAgentToolCallDone};

pub const PREFIX_DETACHED: &str = "[task] 子任务已转后台";
pub const CANCELLED_RESULT: &str = "exit_code=cancelled";
pub const PREFIX_FAILED: &str = "[task] subagent execution failed";

pub fn detached_text(entry: &TaskEntry) -> String {
    format!(
        "{}继续跑, 本回合先消化已经到手的报告、推进其它事情, 不要在这里干等。\n{}\n手上的事做完还要等它时用 {}（任一结束即返回, 等待期间不耗轮次）; 用 {} 回检进度; 静默几分钟且进度行数不再增长才考虑 {}。",
        PREFIX_DETACHED,
        entry.summary_text(),
        AiToolId::ToolTaskWait.as_str(),
        AiToolId::ToolTaskLog.as_str(),
        AiToolId::ToolTaskKill.as_str()
    )
}

pub fn failed_text(entry: &TaskEntry, error: &dyn std::fmt::Display) -> String {
    format!("{}: {}\n{}", PREFIX_FAILED, error, partial_text(entry, 60))
}

pub fn partial_text(entry: &TaskEntry, keep_lines: usize) -> String {
    let progress = entry.progress.lock().clone();
    let rows: Vec<&str> = progress.lines().collect();
    let skip = rows.len().saturating_sub(keep_lines);
    let tail = rows[skip..].join("\n");
    if tail.trim().is_empty() {
        return "(no progress was produced)".to_string();
    }
    format!("[last {} trace lines before termination]\n{}", rows.len() - skip, tail)
}

pub fn push_result(agent_service: &AgentService, parent: &AiRunContext, card_item_id: Option<String>, entry: &TaskEntry) {
    let status = entry.event_status();
    let call_id = entry.call_id.clone();
    let base = RespAgentBase::new(parent.req_id.clone());
    let tool_id = entry.kind.tool_id().as_str().to_string();
    let result = entry.result.lock().clone().unwrap_or_default();
    let run = RespAgentRunBase { run_id: parent.run_id.clone() };
    let done = RespAgentToolCallDone { result, status, tool_id, call_id, run, base };
    let event = RespEventAgent::OnToolCallDone(done);
    agent_service.handler_agent_event(event, entry.chat_id.clone(), None, card_item_id, None);
}
