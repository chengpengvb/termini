use anyhow::Result;
use std::sync::Arc;
use serde_json::json;
use rig::prelude::Tool;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use rig::tool::{DynamicTool, ToolContext};
use crate::module::ai::shared::ToolCallDonePush;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};

pub const TODO_STATUS_PENDING: &str = "pending";
pub const TODO_STATUS_IN_PROGRESS: &str = "in_progress";
pub const TODO_STATUS_COMPLETED: &str = "completed";
pub const TODO_STATUS_CANCELLED: &str = "cancelled";

const MAX_TODOS: usize = 50;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct TodoItem {
    pub content: String,
    pub status: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolTodoWriteArgs {
    pub todos: Vec<TodoItem>,
}

#[base_app(sort = 763)]
#[derive(Clone, Service)]
pub struct ToolTodoWrite {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolTodoWrite {
    fn name(&self) -> &'static str {
        "ToolTodoWrite"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolTodoWrite {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolTodoWrite
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolTodoWrite {
    fn validate(todos: &[TodoItem]) -> Result<(), AiToolError> {
        if todos.is_empty() {
            return Err(AiToolError::Execution("todos is empty".to_string()));
        }
        if todos.len() > MAX_TODOS {
            return Err(AiToolError::Execution(format!("too many todos: {} exceeds limit {}", todos.len(), MAX_TODOS)));
        }
        for todo in todos {
            if todo.content.trim().is_empty() {
                return Err(AiToolError::Execution("todo content is empty".to_string()));
            }
            if !matches!(todo.status.as_str(), TODO_STATUS_PENDING | TODO_STATUS_IN_PROGRESS | TODO_STATUS_COMPLETED | TODO_STATUS_CANCELLED) {
                return Err(AiToolError::Execution(format!("invalid todo status: {}", todo.status)));
            }
        }
        Ok(())
    }

    fn render(todos: &[TodoItem]) -> String {
        let lines: Vec<String> = todos.iter().enumerate().map(|(index, todo)| format!("{}. [{}] {}", index + 1, todo.status, todo.content.trim())).collect();
        format!("任务清单已更新：\n{}", lines.join("\n"))
    }
}

impl Tool for ToolTodoWrite {
    const NAME: &'static str = AiToolId::ToolTodoWrite.as_str();

    type Error = AiToolError;
    type Args = ToolTodoWriteArgs;
    type Output = String;

    fn description(&self) -> String {
        "维护本会话的任务清单。多步骤任务开始前先列出计划；每完成一步立即把该项标为 completed 并提交更新，再继续下一步。todos 是全量清单，每次都要提交完整列表。status 只能是 pending、in_progress、completed、cancelled，同一时间至多一项 in_progress。content 语言遵循 [输出语言]。此工具只记录与展示，不执行任何操作。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "description": "全量任务清单，按执行顺序排列",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": {
                                "type": "string",
                                "description": "任务内容，一句话，语言遵循 [输出语言]"
                            },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed", "cancelled"],
                                "description": "任务状态"
                            }
                        },
                        "required": ["content", "status"]
                    }
                }
            },
            "required": ["todos"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Self::validate(&args.todos)?;
        let rendered = Self::render(&args.todos);
        if let Ok(run_context) = context.require::<Arc<AiRunContext>>() {
            self.chat_session_cache_manager.set_todo_snapshot(&run_context.req_id, rendered.clone());
        }
        context.insert_result(vec![WrapItem::wrap_item(None, rendered)]);
        Ok(String::new())
    }
}
