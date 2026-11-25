use anyhow::Result;
use summer::tracing::warn;
use rig::tool::DynamicTool;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use crate::model::ai_provide::AiProvide;
use crate::module::ai::agent::prompts::render;
use crate::module::ai::provide::ai_agent::AiAgent;
use crate::module::ai::tools::tool_base::ToolBase;
use crate::module::ai::shared::parse_embedded_json;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::agent::agent_build::build_agent_no_thinking;
use crate::module::ai::agent::prompts::LEAD_SHELL_SUPERVISE_SYSTEM;
use crate::module::ai::shared::chat_item_format::{intent_prefix, truncate_chars};
use crate::module::ai::dto::{AiToolId, ShellSuperviseAction, ShellSuperviseDecision};
use crate::module::ai::tools::common::shell::local_shell_registry::ShellProcessEntry;
use crate::module::ai::tools::common::shell::tool_shell_process_log::ToolShellProcessLog;
use crate::module::ai::tools::common::shell::tool_shell_process_kill::ToolShellProcessKill;
use crate::module::ai::tools::common::shell::tool_shell_process_list::ToolShellProcessList;

#[base_app(sort = 768)]
#[derive(Clone, Service)]
pub struct AgentSupervise {
    #[inject(component)]
    provide_factory: ProvideFactory,

    #[inject(component)]
    tool_shell_process_log: ToolShellProcessLog,

    #[inject(component)]
    tool_shell_process_kill: ToolShellProcessKill,

    #[inject(component)]
    tool_shell_process_list: ToolShellProcessList,
}

#[async_trait]
impl BaseApp for AgentSupervise {
    fn name(&self) -> &'static str {
        "AgentSupervise"
    }
}

impl AgentSupervise {
    pub fn build_ai_agent(&self, provide: &AiProvide) -> Result<AiAgent> {
        let preamble = render(
            LEAD_SHELL_SUPERVISE_SYSTEM,
            &[
                ("shell_tools", &self.build_tool_catalog()),
                ("tool_list", AiToolId::ToolShellProcessList.as_str()),
                ("tool_log", AiToolId::ToolShellProcessLog.as_str()),
                ("tool_kill", AiToolId::ToolShellProcessKill.as_str()),
            ],
        );
        build_agent_no_thinking(&self.provide_factory, provide, &preamble, self.build_tools())
    }

    pub fn build_user_prompt(entry: &ShellProcessEntry, shell_id: &str, round: usize, since_seq: u64, tail_lines: usize, user_prompt: &str, goal: Option<&str>) -> String {
        let output = entry.chunk_buffer.lock().clone();
        let start = (since_seq as usize).min(output.len());
        let added = output[start..].chars().count();
        let rows: Vec<&str> = output.lines().collect();
        let skip = rows.len().saturating_sub(tail_lines);
        let tail = rows[skip..].join("\n");
        let body = if tail.is_empty() { "（暂无任何输出）".to_string() } else { tail };
        format!(
            "{}[被监督的本地命令]\n第 {} 轮判定\n终端类型: {}\n{}\n本轮新增输出字数: {}\n\n[最近 {} 行回显]\n{}",
            intent_prefix(user_prompt, goal),
            round,
            shell_id,
            entry.summary_text(),
            added,
            rows.len() - skip,
            body
        )
    }

    pub fn parse_decision(raw: &str) -> Option<ShellSuperviseDecision> {
        #[derive(serde::Deserialize)]
        struct SuperviseOutput {
            action: ShellSuperviseAction,

            #[serde(default)]
            wait_seconds: u64,

            #[serde(default)]
            reason: String,

            #[serde(default)]
            wake_patterns: Vec<String>,
        }
        let Some(out) = parse_embedded_json::<SuperviseOutput>(raw) else {
            warn!("[ai] 监督结论解析失败 raw={}", truncate_chars(raw, 400));
            return None;
        };
        let wake_patterns = out.wake_patterns.into_iter().map(|item| item.trim().to_string()).filter(|item| !item.is_empty()).collect();
        Some(ShellSuperviseDecision { action: out.action, wait_seconds: out.wait_seconds, reason: out.reason.trim().to_string(), wake_patterns })
    }

    fn build_tools(&self) -> Vec<DynamicTool> {
        vec![self.tool_shell_process_list.as_gated_tool(), self.tool_shell_process_log.as_gated_tool(), self.tool_shell_process_kill.as_gated_tool()]
    }

    fn build_tool_catalog(&self) -> String {
        self.build_tools().iter().map(|tool| format!("- {}: {}", tool.name(), tool.definition().description)).collect::<Vec<String>>().join("\n")
    }
}
