use std::sync::Arc;
use serde_json::json;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_notify_mail::NotifyMailService;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};

const MAX_TO: usize = 20;
const CANCELLED_RESULT: &str = "exit_code=cancelled";

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolMailSendArgs {
    pub to: Vec<String>,
    pub subject: String,
    pub body: String,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 770)]
#[derive(Clone, Service)]
pub struct ToolMailSend {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    #[inject(component)]
    notify_mail_service: NotifyMailService,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolMailSend {
    fn name(&self) -> &'static str {
        "ToolMailSend"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolMailSend {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolMailSend
    }

    fn persist(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolMailSend {
    fn parse_to(raw: &[String]) -> Result<Vec<String>, AiToolError> {
        if raw.is_empty() {
            return Err(AiToolError::Execution("to is empty".to_string()));
        }
        let mut emails: Vec<String> = Vec::new();
        for item in raw {
            let email = item.trim().to_string();
            if email.is_empty() {
                return Err(AiToolError::Execution("to contains empty address".to_string()));
            }
            if !Self::is_email(&email) {
                return Err(AiToolError::Execution(format!("invalid email: {}", email)));
            }
            if emails.iter().any(|seen| seen == &email) {
                return Err(AiToolError::Execution(format!("duplicate email: {}", email)));
            }
            emails.push(email);
        }
        if emails.len() > MAX_TO {
            return Err(AiToolError::Execution(format!("too many recipients: {} exceeds {}", emails.len(), MAX_TO)));
        }
        Ok(emails)
    }

    fn is_email(value: &str) -> bool {
        let Some((local, domain)) = value.split_once('@') else {
            return false;
        };
        if local.is_empty() || domain.is_empty() {
            return false;
        }
        if local.contains(' ') || domain.contains(' ') {
            return false;
        }
        domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
    }

    fn parse_text(value: &str, field: &str) -> Result<String, AiToolError> {
        let text = value.trim().to_string();
        if text.is_empty() {
            return Err(AiToolError::Execution(format!("{} is empty", field)));
        }
        Ok(text)
    }

    fn push_cancelled(&self, run_context: &AiRunContext, call_id: &str) -> String {
        let result = CANCELLED_RESULT.to_string();
        self.tool_call_done_push.push_tool_call_done_event(run_context, call_id, result.clone());
        result
    }

    async fn send_all(&self, to: Vec<String>, subject: String, body: String) -> Result<String, AiToolError> {
        self.notify_mail_service.send_mail(&to, &subject, &body).await.map_err(|error| AiToolError::Execution(format!("{:#}", error)))?;
        Ok(format!("sent={}\nok: {}", to.len(), to.join(", ")))
    }
}

impl Tool for ToolMailSend {
    const NAME: &'static str = AiToolId::ToolMailSend.as_str();

    type Args = ToolMailSendArgs;
    type Output = String;
    type Error = AiToolError;

    fn description(&self) -> String {
        "批量发送邮件。用户明确要求把结果/告警/事件/日志等等可发送内容发到一个或多个邮箱时使用。to 一次传入全部地址（最多 20 个），不要拆成多封各调一次。用户没给出邮箱地址时，先 entity_call：entity=config op=list，读 notify_email；有值再调本工具，没有或为空就停并说明没配通知邮箱，不要猜、不要改用其它地址。subject 写邮件标题。body 写完整 HTML 片段，不要 <!DOCTYPE>/<html>/<body>，系统会套进邮件模板；不要 Markdown，不要只写一句「任务完成」。必须同时兼容 PC 和手机：样式全部写在标签 style 上；外层宽度 100%、最大约 600px；栏和数据表用 table，表头有底色，单元格有内边距和边框；关键数字和状态用对比色；正文字号不小于 14px；禁止纯 pre 或无样式堆砌。标题和正文默认语言遵循 [输出语言]；用户点名这封邮件用某种语言时，标题和正文都按指定语言写。定时任务/计划任务里写了「完成后发简报」的，做完立刻发。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "to": {
                    "type": "array",
                    "description": "收件人邮箱，一次传全部，最多 20 个",
                    "minItems": 1,
                    "maxItems": MAX_TO,
                    "items": { "type": "string" }
                },
                "subject": {
                    "type": "string",
                    "description": "邮件标题。默认语言遵循 [输出语言]；用户指定邮件语言时按指定写"
                },
                "body": {
                    "type": "string",
                    "description": "邮件正文，HTML 片段，不要 Markdown，不要完整 html 文档。inline style，table 布局，PC/手机都能看。语言规则同标题"
                }
            },
            "required": ["to", "subject", "body"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let to = Self::parse_to(&args.to)?;
        let body = Self::parse_text(&args.body, "body")?;
        let session_cancel = run_context.cancel_token.clone();
        let subject = Self::parse_text(&args.subject, "subject")?;
        let Some(tool_cancel) = self.chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, &call_id) else {
            return Err(AiToolError::Execution(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };
        tokio::select! {
            biased;
            _ = tool_cancel.cancelled() => {
                let result = self.push_cancelled(&run_context, &call_id);
                context.insert_result(vec![WrapItem::wrap_item(None, result.clone())]);
                Ok(result)
            }
            _ = session_cancel.cancelled() => {
                let result = self.push_cancelled(&run_context, &call_id);
                context.insert_result(vec![WrapItem::wrap_item(None, result.clone())]);
                Ok(result)
            }
            res = self.send_all(to, subject, body) => {
                let result = res?;
                self.tool_call_done_push.push_tool_call_done_event(&run_context, &call_id, result.clone());
                context.insert_result(vec![WrapItem::wrap_item(None, result.clone())]);
                Ok(result)
            }
        }
    }
}
