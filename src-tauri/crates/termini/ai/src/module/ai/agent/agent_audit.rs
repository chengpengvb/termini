use summer::tracing::warn;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use termini_macros::base_app;
use summer::plugin::LazyComponent;
use crate::base::base_app::BaseApp;
use crate::module::ai::shared::ToolCallDonePush;
use crate::module::ai::provide::ai_agent::AiAgent;
use crate::module::ai::tools::base::ToolBaseCollect;
use crate::module::ai::tools::tool_base::ToolDomain;
use crate::module::ai::dto::{AiRunEmit, AuditVerdict};
use crate::module::ai::agent::prompts::LEAD_AUDIT_SYSTEM;
use crate::module::ai::mcp::mcp_client_pool::McpClientPool;
use crate::module::ai::stream::stream_output::StreamOutput;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::agent::agent_base::{AgentBase, AgentRole};
use crate::module::ai::agent::agent_build::build_agent_no_thinking;
use crate::module::ai::shared::{parse_embedded_json, ChatSessionCacheManager};

#[base_app(sort = 768)]
#[derive(Clone, Service)]
pub struct AgentAudit {
    #[inject(component)]
    provide_factory: ProvideFactory,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    mcp_client_pool: LazyComponent<McpClientPool>,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for AgentAudit {
    fn name(&self) -> &'static str {
        "AgentAudit"
    }
}

impl AgentBase for AgentAudit {
    fn role(&self) -> AgentRole {
        AgentRole::Audit
    }

    fn max_turns(&self) -> usize {
        4
    }

    fn emit(&self) -> AiRunEmit {
        AiRunEmit { text: false, reasoning: false, pause: false, status: false, log: false, proxy_error: false }
    }

    fn tool_concurrency(&self) -> usize {
        2
    }

    fn build_ai_agent(&self, req_id: &str) -> Result<AiAgent> {
        let entry = self.chat_session_cache_manager.get(req_id).ok_or_else(|| anyhow!("unknown req_id: {}", req_id))?;
        let mcp_client_pool = self.mcp_client_pool.get().map_err(|error| anyhow!(error.to_string()))?;
        let tools = ToolBaseCollect::collect(&entry.tools, &mcp_client_pool, ToolDomain::Audit, &self.tool_call_done_push);
        build_agent_no_thinking(&self.provide_factory, &entry.light_provide(), LEAD_AUDIT_SYSTEM, tools)
    }
}

impl AgentAudit {
    pub fn parse_verdict(raw: &str) -> AuditVerdict {
        #[derive(serde::Deserialize)]
        struct AuditOutput {
            #[serde(default)]
            allow: bool,

            #[serde(default)]
            reason: String,

            #[serde(default)]
            checked: String,
        }
        let Some(out) = parse_embedded_json::<AuditOutput>(raw) else {
            warn!("[ai] 审核输出无法解析 raw={}", raw.chars().take(600).collect::<String>());
            return AuditVerdict::reject("auto audit returned no parseable verdict", "");
        };
        let checked = out.checked.trim().to_string();
        if out.allow {
            return AuditVerdict::allow(checked);
        }
        if out.reason.trim().is_empty() {
            return AuditVerdict::reject("auto audit rejected without a reason", checked);
        }
        AuditVerdict::reject(out.reason.trim(), checked)
    }
}

impl StreamOutput for AuditVerdict {
    fn from_stream_text(raw: &str) -> Self {
        AgentAudit::parse_verdict(raw)
    }
}
