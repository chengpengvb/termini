use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use std::time::Duration;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use serde_json::{json, Value};
use crate::util::id_factory::uuid;
use summer::tracing::{info, warn};
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::model::ai_provide::AiProvide;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ChatFileStore;
use crate::service::service_base::BaseService;
use crate::module::ai::shared::ToolCallDonePush;
use crate::service::service_agent::AgentService;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::shared::ToolCallChunkPush;
use crate::module::ai::shared::GeneratedImagePush;
use crate::service::service_config::ConfigService;
use crate::service::service_base_api::BaseApiService;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_ai_provide::AiProvideService;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::shared::chat_file_store::CHAT_IMAGES_DIR;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::tool_base::{ToolBase, ToolDomain};
use crate::module::ai::tools::common::task::session::task_result;
use crate::module::ai::tools::common::task::session::task_coordinator::{TaskCoordinator, TaskOutcome};
use crate::module::ai::shared::generated_image::{save_generated_image, save_image_base64, GeneratedImage};
use crate::module::ai::tools::common::task::session::task_registry::{TaskEntry, TaskKind, TaskRegistry, TaskState};

const IMAGE_COUNT: u8 = 1;
const BLOCK_MAX_SECS: i64 = 90;
const TRANSPORT_RETRY: usize = 1;
const REQUEST_MAX_SECS: u64 = 600;
const TASK_PROMPT_MAX_CHARS: usize = 60;
const IMAGES_PATH: &str = "/images/generations";
const PREFIX_FAILED: &str = "generation failed";
const CANCELLED_RESULT: &str = "generation cancelled";
const PREFIX_DETACHED: &str = "生成耗时较长, 已转后台继续";

enum ImageSource {
    Url(String),
    Base64(String),
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolImageGenerateArgs {
    pub prompt: String,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 766)]
#[derive(Clone, Service)]
pub struct ToolImageGenerate {
    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    task_registry: TaskRegistry,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    chat_file_store: ChatFileStore,

    #[inject(component)]
    task_coordinator: TaskCoordinator,

    #[inject(component)]
    ai_provide_service: AiProvideService,

    #[inject(component)]
    tool_call_chunk_push: ToolCallChunkPush,

    #[inject(component)]
    generated_image_push: GeneratedImagePush,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolImageGenerate {
    fn name(&self) -> &'static str {
        "ToolImageGenerate"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolImageGenerate {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolImageGenerate
    }

    fn persist(&self) -> bool {
        true
    }

    fn deferred(&self) -> bool {
        true
    }

    fn domains(&self) -> &'static [ToolDomain] {
        &[ToolDomain::Main]
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl BaseApiService for ToolImageGenerate {}

impl ToolImageGenerate {
    async fn generate(&self, run_context: &Arc<AiRunContext>, call_id: &str, chat_id: String, prompt: String) -> Result<String, AiToolError> {
        let session_cancel = run_context.cancel_token.clone();
        let Some(tool_cancel) = self.chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, call_id) else {
            return Err(AiToolError::Execution(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };
        let provide = self.resolve_provide().await?;
        let task_kind = TaskKind::ImageGenerate;
        let tool_name = task_kind.tool_id().as_str();
        let task_prompt = format!("{} {}", tool_name, first_chars(&prompt, TASK_PROMPT_MAX_CHARS));

        let parent_req_id = self.chat_session_cache_manager.parent_req_id(&run_context.req_id).unwrap_or_else(|| run_context.req_id.clone());
        let entry = TaskEntry::new(task_kind, parent_req_id, chat_id, uuid(), call_id.to_string(), task_prompt, session_cancel.child_token());
        self.task_registry.insert(entry.clone());
        let card_item_id = self.chat_session_cache_manager.get_tool_item_id(&run_context.req_id, call_id);

        let worker = self.clone();
        let job_entry = entry.clone();
        let job_context = Arc::clone(run_context);
        tokio::spawn(async move {
            let outcome = worker.run_job(&job_context, &job_entry, &provide, prompt).await;
            if job_entry.cancel.is_cancelled() {
                job_entry.finish(TaskState::Cancelled, Some(format!("[{}] {}", tool_name, CANCELLED_RESULT)));
            } else {
                match outcome {
                    Ok(text) => job_entry.finish(TaskState::Done, Some(text)),
                    Err(error) => job_entry.finish(TaskState::Failed, Some(format!("[{}] {}: {}\n{}", tool_name, PREFIX_FAILED, error, task_result::partial_text(&job_entry, 20)))),
                }
            }
            task_result::push_result(&worker.agent_service, &job_context, card_item_id, &job_entry);
        });

        match self.task_coordinator.join(&run_context.req_id, session_cancel, entry.clone(), tool_cancel, BLOCK_MAX_SECS).await {
            TaskOutcome::Detached => Ok(Self::detached_text(&entry)),
            TaskOutcome::Cancelled => {
                entry.cancel.cancel();
                entry.finish(TaskState::Cancelled, Some(format!("[{}] {}", tool_name, CANCELLED_RESULT)));
                Ok(format!("[{}] {}", tool_name, CANCELLED_RESULT))
            }
            TaskOutcome::Finished => {
                let result = entry.result.lock().clone().unwrap_or_default();
                self.task_registry.remove(&entry.chat_id, &entry.task_id);
                if entry.state() == TaskState::Failed {
                    return Err(AiToolError::Execution(result));
                }
                Ok(result)
            }
        }
    }

    fn detached_text(entry: &TaskEntry) -> String {
        format!(
            "[{}] {}, 图片生成后会直接显示在界面并回推到卡片, 本回合先推进其它事情。\n{}\n手上没别的事时用 {} 等它结束, 之后 {} 传 include_result=true 取结果(含图片路径); 要中止用 {}。",
            entry.kind.tool_id().as_str(),
            PREFIX_DETACHED,
            entry.summary_text(),
            AiToolId::ToolTaskWait.as_str(),
            AiToolId::ToolTaskLog.as_str(),
            AiToolId::ToolTaskKill.as_str()
        )
    }

    async fn resolve_provide(&self) -> Result<AiProvide, AiToolError> {
        let Some(config) = self.config_service.select_config().await else {
            return Err(AiToolError::Execution("default config not found".to_string()));
        };
        let provide_id = match config.resolve_image_gen_provide_id() {
            Some(id) => id,
            None if !config.image_gen_use_cloud => return Err(AiToolError::Execution("image generation is set to a local model in settings, but no model is selected".to_string())),
            None => {
                self.config_service.refresh_system_config().await.map_err(|error| AiToolError::Execution(format!("failed to refresh system config: {}", error)))?;
                let Some(refreshed) = self.config_service.select_config().await else {
                    return Err(AiToolError::Execution("default config not found".to_string()));
                };
                refreshed.resolve_image_gen_provide_id().ok_or_else(|| AiToolError::Execution("server has no image generation model configured".to_string()))?
            }
        };
        if let Some(shared) = self.ai_provide_service.find_by_id(provide_id.clone()).await.map_err(|error| AiToolError::Execution(error.to_string()))? {
            let provide = shared.read().clone();
            return Ok(provide);
        }
        if let Err(error) = self.ai_provide_service.refresh_cloud().await {
            warn!("[ai] 刷新云端模型列表失败: {error:#}");
        }
        let Some(shared) = self.ai_provide_service.find_by_id(provide_id.clone()).await.map_err(|error| AiToolError::Execution(error.to_string()))? else {
            return Err(AiToolError::Execution(format!("unknown image generation model provide_id: {}", provide_id)));
        };
        let provide = shared.read().clone();
        Ok(provide)
    }

    async fn run_job(&self, run_context: &Arc<AiRunContext>, entry: &TaskEntry, provide: &AiProvide, prompt: String) -> Result<String, String> {
        let sources = self.ask_guarded(entry, provide, prompt).await?;
        if sources.is_empty() {
            return Err("the model returned no images".to_string());
        }
        let dir = self.chat_file_store.ensure_dir_for_req(&run_context.req_id, CHAT_IMAGES_DIR).await.map_err(|error| format!("failed to resolve chat image dir: {}", error))?;
        let mut images: Vec<GeneratedImage> = Vec::new();
        for source in sources {
            if entry.cancel.is_cancelled() {
                return Err(CANCELLED_RESULT.to_string());
            }
            let saved = match source {
                ImageSource::Url(url) => save_generated_image(&dir, &url).await,
                ImageSource::Base64(payload) => save_image_base64(&dir, &payload).await,
            };
            let saved = saved.map_err(|error| format!("failed to save generated image: {}", error))?;
            info!("[ai] 图片已保存 model={} path={} media_type={}", provide.model, saved.path, saved.media_type);
            self.generated_image_push.push(run_context, &saved);
            self.push_delta(run_context, entry, &format!("[已保存] {}\n", saved.path));
            images.push(saved);
        }
        let list = images.iter().map(|image| format!("- {}", image.path)).collect::<Vec<String>>().join("\n");
        Ok(format!("已生成 {} 张图片, 已保存为本地文件并显示在界面, 需要引用时直接给路径:\n{}", images.len(), list))
    }

    fn push_delta(&self, run_context: &AiRunContext, entry: &TaskEntry, delta: &str) {
        entry.append_progress(delta);
        if let Err(error) = self.tool_call_chunk_push.push(run_context, &entry.call_id, entry.kind.tool_id().as_str(), delta.to_string()) {
            warn!("[ai] 推送生成进度失败 call_id={} error={:#}", entry.call_id, error);
        }
    }

    async fn ask_guarded(&self, entry: &TaskEntry, provide: &AiProvide, prompt: String) -> Result<Vec<ImageSource>, String> {
        let request = tokio::time::timeout(Duration::from_secs(REQUEST_MAX_SECS), self.ask(provide, prompt));
        tokio::select! {
            output = request => match output {
                Ok(result) => result,
                Err(_) => Err(format!("single generation request did not finish within {}s, giving up", REQUEST_MAX_SECS)),
            },
            _ = entry.cancel.cancelled() => Err(CANCELLED_RESULT.to_string()),
        }
    }

    async fn ask(&self, provide: &AiProvide, prompt: String) -> Result<Vec<ImageSource>, String> {
        let url = format!("{}{}", provide.url.trim().trim_end_matches('/'), IMAGES_PATH);
        let mut body = ProvideFactory::id_params(provide);
        body.insert("model".to_string(), json!(provide.model));
        body.insert("prompt".to_string(), json!(prompt));
        body.insert("n".to_string(), json!(IMAGE_COUNT));
        let body = Value::Object(body);
        let key = provide.key.clone().unwrap_or_default();
        let mut attempt = 0usize;
        loop {
            let client = ProvideFactory::http_client().map_err(|error| error.to_string())?;
            let started = Instant::now();
            let response = client.post(&url).bearer_auth(&key).json(&body).send().await;
            match response {
                Ok(response) => {
                    Self::api_observe_config_version(response.headers());
                    let status = response.status();
                    let text = response.text().await.map_err(|error| format!("failed to read image generation response: {}", error))?;
                    if !status.is_success() {
                        warn!("[ai] 生成请求失败 model={} 耗时={:.1}s status={} body={}", provide.model, started.elapsed().as_secs_f64(), status, text);
                        return Err(format!("image generation model request failed: HTTP {} {}", status.as_u16(), text));
                    }
                    let sources = Self::parse_images(&text)?;
                    info!("[ai] 生成请求完成 model={} 耗时={:.1}s 图片={}", provide.model, started.elapsed().as_secs_f64(), sources.len());
                    return Ok(sources);
                }
                Err(error) if attempt < TRANSPORT_RETRY && (error.is_connect() || error.is_timeout()) => {
                    attempt += 1;
                    warn!("[ai] 生成请求传输失败, 重置连接后重试 attempt={} model={} 耗时={:.1}s error={:#}", attempt, provide.model, started.elapsed().as_secs_f64(), error);
                    ProvideFactory::reset_http_client();
                }
                Err(error) => {
                    warn!("[ai] 生成请求失败 model={} 耗时={:.1}s error={:#}", provide.model, started.elapsed().as_secs_f64(), error);
                    return Err(format!("image generation model request failed: {}", error));
                }
            }
        }
    }

    fn parse_images(text: &str) -> Result<Vec<ImageSource>, String> {
        let root: Value = serde_json::from_str(text).map_err(|error| format!("image generation response is not JSON: {} body: {}", error, text))?;
        let Some(data) = root.get("data").and_then(Value::as_array) else {
            return Err(format!("image generation response is missing the data array: {}", text));
        };
        let mut sources = Vec::new();
        for item in data {
            if let Some(payload) = item.get("b64_json").and_then(Value::as_str) {
                sources.push(ImageSource::Base64(payload.to_string()));
            } else if let Some(url) = item.get("url").and_then(Value::as_str) {
                sources.push(ImageSource::Url(url.to_string()));
            } else {
                return Err(format!("image generation response data item has neither b64_json nor url: {}", item));
            }
        }
        Ok(sources)
    }
}

impl Tool for ToolImageGenerate {
    const NAME: &'static str = AiToolId::ToolImageGenerate.as_str();

    type Error = AiToolError;
    type Args = ToolImageGenerateArgs;
    type Output = String;

    fn description(&self) -> String {
        "根据文字描述生成图片，由设置里配置的图片生成模型（默认云端下发）走标准 images/generations 接口完成。生成的图片保存为本应用所在宿主机上的本地文件并直接显示在对话界面，返回里给出文件路径（数据库只记路径，不存图片数据）；之后要引用、用 image_read 复读或交给 docx_write/pdf_write 时直接用该路径。prompt 写清主体、风格、构图、色调、比例等，一次调用生成一组；要修改就带着改动点重新调用。超过约 90 秒转后台并返回 task_id，用 task_wait 等结果、task_log 传 include_result=true 取结果、task_kill 中止。未配置图片生成模型时会直接报错，如实告知用户即可。".to_string()
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "prompt": { "type": "string", "description": "要生成的图片描述" }
            },
            "required": ["prompt"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let Some(chat_id) = self.chat_session_cache_manager.chat_id(&run_context.req_id) else {
            return Err(AiToolError::Execution(format!("unknown req_id: {}", run_context.req_id)));
        };
        let prompt = args.prompt.trim().to_string();
        if prompt.is_empty() {
            return Err(AiToolError::Execution("prompt is empty".to_string()));
        }
        let body = self.generate(&run_context, &call_id, chat_id, prompt).await?;
        context.insert_result(vec![WrapItem::wrap_item(None, body.clone())]);
        Ok(body)
    }
}

fn first_chars(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max {
        return trimmed.to_string();
    }
    trimmed.chars().take(max).collect::<String>() + "…"
}
