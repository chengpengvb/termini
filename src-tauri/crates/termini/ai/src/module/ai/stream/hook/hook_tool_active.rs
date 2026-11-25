use rig::completion::Document;
use crate::module::ai::dto::AiToolId;
use crate::module::ai::shared::ToolActivationStore;
use rig::agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, RequestPatch, StepEventKind};

pub struct ToolActiveHook {
    pub enabled: bool,
    pub req_id: String,
    pub chat_id: String,
    pub tool_activation_store: ToolActivationStore,
}

impl AgentHook for ToolActiveHook {
    async fn on_completion_call(&self, _ctx: &HookContext, _event: CompletionCallEvent<'_>) -> CompletionCallAction {
        if !self.enabled {
            return CompletionCallAction::Continue;
        }
        let Some(catalog) = self.tool_activation_store.catalog(&self.req_id) else {
            return CompletionCallAction::Continue;
        };
        if catalog.deferred.is_empty() {
            return CompletionCallAction::Continue;
        }
        let Some(handle_names) = self.tool_activation_store.handle_names(&self.req_id) else {
            return CompletionCallAction::Continue;
        };
        let activated = self.tool_activation_store.activated(&self.chat_id);
        let mut active: Vec<String> = catalog.always.iter().filter(|name| handle_names.contains(*name)).cloned().collect();
        active.extend(catalog.deferred.iter().filter(|tool| activated.contains(&tool.name) && handle_names.contains(&tool.name)).map(|tool| tool.name.clone()));
        let pending: Vec<String> = catalog.deferred.iter().filter(|tool| !activated.contains(&tool.name)).map(|tool| format!("- {} — {}", tool.name, tool.summary)).collect();
        let patch = RequestPatch::new().active_tools(active);
        if pending.is_empty() {
            return CompletionCallAction::patch(patch);
        }
        let text = format!("[可检索工具]\n以下工具当前不可直接调用；需要时先用 {} 按关键词检索，命中的工具从下一轮起可用：\n{}", AiToolId::ToolToolSearch.as_str(), pending.join("\n"));
        CompletionCallAction::patch(patch.extra_context(vec![Document { id: "deferred-tools".to_string(), text, additional_props: Default::default() }]))
    }

    fn observes(&self, kind: StepEventKind) -> bool {
        matches!(kind, StepEventKind::CompletionCall)
    }
}
