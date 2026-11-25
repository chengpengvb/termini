use rig::completion::Document;
use crate::module::ai::derived::DerivedStore;
use crate::module::ai::shared::ChatSessionCacheManager;
use rig::agent::{AgentHook, CompletionCallAction, CompletionCallEvent, HookContext, RequestPatch, StepEventKind};

pub struct SessionContextHook {
    pub req_id: String,
    pub derived_store: DerivedStore,
    pub chat_session_cache_manager: ChatSessionCacheManager,
}

impl SessionContextHook {
    fn build_documents(&self) -> Vec<Document> {
        let mut documents: Vec<Document> = Vec::new();
        for (path, content) in self.chat_session_cache_manager.agents_docs(&self.req_id) {
            documents.push(Document { id: format!("agents-md:{}", path), text: format!("[项目规范 {}]\n{}", path, content), additional_props: Default::default() });
        }
        if let Some(todo) = self.chat_session_cache_manager.todo_snapshot(&self.req_id) {
            documents.push(Document {
                id: "todo-snapshot".to_string(),
                text: format!("[任务清单]\n这是你之前用 todo_write 记录的任务清单，仍然有效。按它继续推进，每完成一步用 todo_write 更新状态。条目语言遵循 [输出语言]：\n{}", todo),
                additional_props: Default::default(),
            });
        }
        if let Some(chat_id) = self.chat_session_cache_manager.get(&self.req_id).map(|entry| entry.context.ai_chat_id.clone()) {
            if let Some(materials) = self.derived_store.render_materials(&chat_id) {
                documents.push(Document { id: "materials".to_string(), text: materials, additional_props: Default::default() });
            }
        }
        let paths = self.chat_session_cache_manager.changed_paths(&self.req_id);
        if !paths.is_empty() {
            documents.push(Document {
                id: "file-change-notice".to_string(),
                text: format!("[本轮改动]\n已修改 {} 个文件：\n{}\n收尾前必须验证改动（构建/测试/lint 至少一项），验证失败要么修复要么在答复里如实说明。", paths.len(), paths.join("\n")),
                additional_props: Default::default(),
            });
        }
        documents
    }
}

impl AgentHook for SessionContextHook {
    async fn on_completion_call(&self, _ctx: &HookContext, _event: CompletionCallEvent<'_>) -> CompletionCallAction {
        let documents = self.build_documents();
        if documents.is_empty() {
            return CompletionCallAction::Continue;
        }
        CompletionCallAction::patch(RequestPatch::new().extra_context(documents))
    }

    fn observes(&self, kind: StepEventKind) -> bool {
        matches!(kind, StepEventKind::CompletionCall)
    }
}
