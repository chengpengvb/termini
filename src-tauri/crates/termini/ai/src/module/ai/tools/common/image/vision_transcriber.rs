use anyhow::Result;
use base64::Engine;
use std::sync::Arc;
use std::path::PathBuf;
use std::time::Instant;
use std::time::Duration;
use summer::plugin::Service;
use async_trait::async_trait;
use rig::completion::Message;
use termini_macros::base_app;
use crate::model::config::Config;
use crate::util::id_factory::uuid;
use summer::tracing::{info, warn};
use crate::module::ai::dto::AiToolId;
use crate::model::ai_provide::AiProvide;
use crate::module::ai::dto::AiRunContext;
use base64::engine::general_purpose::STANDARD;
use crate::service::service_base::BaseService;
use crate::service::service_agent::AgentService;
use crate::base::base_app::{AppContext, BaseApp};
use crate::module::ai::shared::ToolCallChunkPush;
use crate::service::service_config::ConfigService;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::common::image::transcript;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::service::service_ai_provide::AiProvideService;
use crate::module::ai::tools::common::image::layout_lines;
use rig::message::{ImageMediaType, MimeType, UserContent};
use crate::module::ai::derived::{DerivedKind, DerivedStore};
use crate::module::ai::agent::prompts::LEAD_IMAGE_READ_SYSTEM;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::tools::common::task::session::task_result;
use crate::module::ai::tools::common::image::part_store::PartStore;
use crate::module::ai::tools::common::image::image_prepare::{PrepareOptions, PreparedImage};
use crate::module::ai::tools::common::image::repeat_guard::{truncate_chars, RepeatGuard, RepeatHit};
use crate::module::ai::tools::common::task::session::task_coordinator::{TaskCoordinator, TaskOutcome};
use crate::module::ai::tools::common::task::session::task_registry::{TaskEntry, TaskKind, TaskRegistry, TaskState};
use crate::module::ai::agent::agent_build::{build_agent_no_thinking, is_transport_error, stream_prompt_text_until, StreamControl};

const VISION_TRANSPORT_RETRY: usize = 1;
const VISION_BLOCK_MAX_SECS: i64 = 90;
const VISION_REQUEST_MAX_SECS: u64 = 600;
const PREFIX_FAILED: &str = "transcription failed";
const CANCELLED_RESULT: &str = "read cancelled";
const REQUEST_PROMPT: &str = "识别这张图片。";
const PREFIX_DETACHED: &str = "识别耗时较长, 已转后台继续";

pub struct TranscribeUnit {
    pub label: Option<String>,
    pub images: Vec<PreparedImage>,
}

pub struct TranscribeSource {
    pub key: String,
    pub path: PathBuf,
    pub variant: String,
    pub source_len: u64,
    pub kind: DerivedKind,
    pub source_hash: String,
    pub task_kind: TaskKind,
    pub label: Option<String>,
    pub units: Vec<TranscribeUnit>,
}

struct VisionJob {
    provide: AiProvide,
    derived_root: PathBuf,
    source: TranscribeSource,
}

#[base_app(sort = 765)]
#[derive(Clone, Service)]
pub struct VisionTranscriber {
    #[inject(component)]
    derived_store: DerivedStore,

    #[inject(component)]
    agent_service: AgentService,

    #[inject(component)]
    task_registry: TaskRegistry,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    provide_factory: ProvideFactory,

    #[inject(component)]
    task_coordinator: TaskCoordinator,

    #[inject(component)]
    ai_provide_service: AiProvideService,

    #[inject(component)]
    tool_call_chunk_push: ToolCallChunkPush,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for VisionTranscriber {
    fn name(&self) -> &'static str {
        "VisionTranscriber"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        Ok(())
    }
}

impl VisionTranscriber {
    pub async fn prepare_options(&self) -> Result<PrepareOptions, AiToolError> {
        let config = self.select_vision_config().await?;
        Ok(PrepareOptions {
            max_pixels: config.vision_max_pixels.filter(|value| *value > 0).map(|value| value as u32),
            jpeg_quality: config.vision_jpeg_quality.filter(|value| (50..=100).contains(value)).map(|value| value as u8),
        })
    }

    async fn select_vision_config(&self) -> Result<Config, AiToolError> {
        let Some(config) = self.config_service.select_config().await else {
            return Err(AiToolError::Execution("default config not found".to_string()));
        };
        if !config.vision_use_cloud || config.resolve_vision_provide_id().is_some() {
            return Ok(config);
        }
        self.config_service.refresh_system_config().await.map_err(|error| AiToolError::Execution(format!("failed to refresh system config: {}", error)))?;
        let Some(refreshed) = self.config_service.select_config().await else {
            return Err(AiToolError::Execution("default config not found".to_string()));
        };
        Ok(refreshed)
    }

    pub async fn transcribe(&self, run_context: &Arc<AiRunContext>, call_id: &str, chat_id: String, source: TranscribeSource) -> Result<String, AiToolError> {
        if source.units.iter().all(|unit| unit.images.is_empty()) {
            return Err(AiToolError::Execution(format!("no recognizable image content: {}", source.path.display())));
        }
        let session_cancel = run_context.cancel_token.clone();
        let Some(tool_cancel) = self.chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, call_id) else {
            return Err(AiToolError::Execution(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };
        let provide = self.resolve_provide().await?;
        let derived_root = DerivedStore::root().map_err(|error| AiToolError::Execution(error.to_string()))?;
        let task_kind = source.task_kind;
        let tool_name = task_kind.tool_id().as_str();
        let task_prompt = format!("{} {}", tool_name, source.path.display());
        let job = VisionJob { source, provide, derived_root };

        let parent_req_id = self.chat_session_cache_manager.parent_req_id(&run_context.req_id).unwrap_or_else(|| run_context.req_id.clone());
        let entry = TaskEntry::new(task_kind, parent_req_id, chat_id, uuid(), call_id.to_string(), task_prompt, session_cancel.child_token());
        self.task_registry.insert(entry.clone());
        let card_item_id = self.chat_session_cache_manager.get_tool_item_id(&run_context.req_id, call_id);

        let worker = self.clone();
        let job_entry = entry.clone();
        let job_context = Arc::clone(run_context);
        tokio::spawn(async move {
            let outcome = worker.run_job(&job_context, &job_entry, job).await;
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

        match self.task_coordinator.join(&run_context.req_id, session_cancel, entry.clone(), tool_cancel, VISION_BLOCK_MAX_SECS).await {
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
            "[{}] {}, 结果出来后会保存为派生文档并回推到卡片, 本回合先推进其它事情。\n{}\n用 {} 看已识别出的文字(默认最近 30 行), 发现它在重复同一段话或明显跑偏就 {} 打断后重新调用(已完成的分段会保留, 从断点继续); 手上没别的事时用 {} 等它结束, 之后 {} 传 include_result=true 取结果开头(含 md 路径, 全文用 file_read)。",
            entry.kind.tool_id().as_str(),
            PREFIX_DETACHED,
            entry.summary_text(),
            AiToolId::ToolTaskLog.as_str(),
            AiToolId::ToolTaskKill.as_str(),
            AiToolId::ToolTaskWait.as_str(),
            AiToolId::ToolTaskLog.as_str()
        )
    }

    async fn resolve_provide(&self) -> Result<AiProvide, AiToolError> {
        let config = self.select_vision_config().await?;
        let provide_id = match config.resolve_vision_provide_id() {
            Some(id) => id,
            None if !config.vision_use_cloud => return Err(AiToolError::Execution("image reading is set to a local vision model in settings, but no model is selected".to_string())),
            None => return Err(AiToolError::Execution("server has no vision model configured for image reading".to_string())),
        };
        if let Some(shared) = self.ai_provide_service.find_by_id(provide_id.clone()).await.map_err(|error| AiToolError::Execution(error.to_string()))? {
            let provide = shared.read().clone();
            return Ok(provide);
        }
        if let Err(error) = self.ai_provide_service.refresh_cloud().await {
            warn!("[ai] 刷新云端模型列表失败: {error:#}");
        }
        let Some(shared) = self.ai_provide_service.find_by_id(provide_id.clone()).await.map_err(|error| AiToolError::Execution(error.to_string()))? else {
            return Err(AiToolError::Execution(format!("unknown vision model provide_id: {}", provide_id)));
        };
        let provide = shared.read().clone();
        Ok(provide)
    }

    fn build_vision_message(image: &PreparedImage, index: usize, total: usize) -> Result<Message, String> {
        let mut text = REQUEST_PROMPT.to_string();
        if total > 1 {
            text.push_str(&format!("（原图较大，已从上到下切成 {} 个连续片段，这是第 {} 片，请完整输出这一片里从最顶端到最底端的全部内容，顶部被切断的半行也照写。）", total, index + 1));
        }
        let parts = vec![UserContent::text(text), UserContent::image_base64(STANDARD.encode(&image.bytes), ImageMediaType::from_mime_type(image.mime), None)];
        Ok(Message::User { content: parts })
    }

    async fn run_job(&self, run_context: &Arc<AiRunContext>, entry: &TaskEntry, job: VisionJob) -> Result<String, String> {
        let source = &job.source;
        let total: usize = source.units.iter().map(|unit| unit.images.len()).sum();
        let derived_dir = DerivedStore::dir_for(&source.key).map_err(|error| error.to_string())?;
        let parts = PartStore::open(derived_dir, &source.variant, &source.source_hash, total);
        let existing = parts.load();
        let resumed = existing.iter().filter(|part| part.is_some()).count();
        if resumed > 0 {
            info!("[ai] 从断点继续 path={} done={}/{}", source.path.display(), resumed, total);
            self.push_delta(run_context, entry, &format!("[断点续传: 已有 {}/{} 段完成, 跳过重识别]\n", resumed, total));
        }
        let mut pages: Vec<(String, String)> = Vec::new();
        let mut whole: Vec<String> = Vec::new();
        let mut request_index = 0usize;
        let mut done = resumed;
        for unit in &source.units {
            if entry.cancel.is_cancelled() {
                return Err(CANCELLED_RESULT.to_string());
            }
            if let Some(label) = unit.label.as_deref() {
                self.push_delta(run_context, entry, &format!("{}\n", label));
            }
            let mut unit_texts: Vec<String> = Vec::new();
            for (image_index, image) in unit.images.iter().enumerate() {
                if entry.cancel.is_cancelled() {
                    return Err(CANCELLED_RESULT.to_string());
                }
                let text = match existing.get(request_index).and_then(|part| part.clone()) {
                    Some(saved) => saved,
                    None => {
                        let tiles = unit.images.len();
                        let outcome = self.transcribe_tile(run_context, entry, &job.provide, image, image_index, tiles).await;
                        let text = outcome.map_err(|error| format!("{} ({}/{} segments completed and checkpointed; calling again resumes from the checkpoint)", error, done, total))?;
                        parts.save(request_index, &text)?;
                        done += 1;
                        text
                    }
                };
                unit_texts.push(text);
                request_index += 1;
            }
            let unit_text = layout_lines::stitch(&unit_texts);
            whole.push(unit_text.clone());
            pages.push((unit.label.clone().unwrap_or_default(), unit_text));
        }
        let labeled = source.units.iter().any(|unit| unit.label.is_some());
        let full = if labeled { transcript::compose_pages(&pages) } else { transcript::compose(&whole.join("\n")) };
        let source_path = source.path.to_string_lossy();
        let hash = Some(source.source_hash.clone());
        let len = Some(source.source_len);
        let model = Some(job.provide.model.clone());
        let saved = self.derived_store.save(&source.key, &source.variant, source.kind, &source_path, hash, len, source.label.clone(), None, model, &full);
        let derived = saved.map_err(|error| format!("failed to save transcription result: {}", error))?;
        parts.clear();
        self.derived_store.record_material(&entry.chat_id, &derived);
        Ok(format!("{}\n\n{}", derived.header(&job.derived_root), full))
    }

    async fn transcribe_tile(&self, run_context: &Arc<AiRunContext>, entry: &TaskEntry, provide: &AiProvide, image: &PreparedImage, index: usize, total: usize) -> Result<String, String> {
        let message = Self::build_vision_message(image, index, total)?;
        let raw = self.ask_vision_guarded(run_context, entry, provide, message).await?;
        Ok(layout_lines::clean(&raw))
    }

    fn push_delta(&self, run_context: &AiRunContext, entry: &TaskEntry, delta: &str) {
        entry.append_progress(delta);
        if let Err(error) = self.tool_call_chunk_push.push(run_context, &entry.call_id, entry.kind.tool_id().as_str(), delta.to_string()) {
            warn!("[ai] 推送解析进度失败 call_id={} error={:#}", entry.call_id, error);
        }
    }

    async fn ask_vision_guarded(&self, run_context: &Arc<AiRunContext>, entry: &TaskEntry, provide: &AiProvide, message: Message) -> Result<String, String> {
        let request = tokio::time::timeout(Duration::from_secs(VISION_REQUEST_MAX_SECS), self.ask_vision(run_context, entry, provide, message));
        tokio::select! {
            output = request => match output {
                Ok(result) => result,
                Err(_) => Err(format!("single vision request did not finish within {}s, giving up", VISION_REQUEST_MAX_SECS)),
            },
            _ = entry.cancel.cancelled() => Err(CANCELLED_RESULT.to_string()),
        }
    }

    async fn ask_vision(&self, run_context: &Arc<AiRunContext>, entry: &TaskEntry, provide: &AiProvide, message: Message) -> Result<String, String> {
        let mut attempt = 0usize;
        loop {
            let agent = build_agent_no_thinking(&self.provide_factory, provide, LEAD_IMAGE_READ_SYSTEM, vec![]).map_err(|error| error.to_string())?;
            let started = Instant::now();
            let mut guard = RepeatGuard::new();
            let mut hit: Option<RepeatHit> = None;
            let on_delta = |delta: &str| {
                self.push_delta(run_context, entry, delta);
                match guard.feed(delta) {
                    Some(found) => {
                        hit = Some(found);
                        StreamControl::Stop
                    }
                    None => StreamControl::Continue,
                }
            };
            let streamed = stream_prompt_text_until(agent, message.clone(), on_delta).await;
            let received = guard.total_chars();
            match streamed {
                Ok(streamed) if streamed.stopped => {
                    let Some(hit) = hit else {
                        return Err("vision model output was cut off early".to_string());
                    };
                    let kept = truncate_chars(&streamed.text, hit.keep_chars).chars().count();
                    let seconds = started.elapsed().as_secs_f64();
                    warn!("[ai] 视觉模型输出陷入重复, 已掐断 model={} 耗时={:.1}s 输出={}字 保留={}字 重复片段={:?} 次数={}", provide.model, seconds, received, kept, hit.unit, hit.times);
                    return Err(format!(
                        "vision model output got stuck repeating (segment \"{}\" appeared {} times in a row); cut off, this segment was not saved",
                        first_chars(&hit.unit, 40),
                        hit.times
                    ));
                }
                Ok(streamed) => {
                    info!("[ai] 视觉模型解析完成 model={} 耗时={:.1}s 输出={}字", provide.model, started.elapsed().as_secs_f64(), streamed.text.chars().count());
                    return Ok(streamed.text);
                }
                Err(error) if attempt < VISION_TRANSPORT_RETRY && is_transport_error(&error) => {
                    attempt += 1;
                    warn!("[ai] 视觉请求传输失败, 重置连接后重试 attempt={} model={} 耗时={:.1}s 已收={}字 error={:#}", attempt, provide.model, started.elapsed().as_secs_f64(), received, error);
                    ProvideFactory::reset_http_client();
                    self.push_delta(run_context, entry, &format!("\n[传输中断, 已收到的 {} 字作废, 重试本片 {}/{}]\n", received, attempt, VISION_TRANSPORT_RETRY));
                }
                Err(error) => {
                    warn!("[ai] 视觉模型解析失败 model={} 耗时={:.1}s error={:#}", provide.model, started.elapsed().as_secs_f64(), error);
                    return Err(format!("vision model transcription failed: {}", error));
                }
            }
        }
    }
}

fn first_chars(text: &str, max: usize) -> String {
    let trimmed = text.trim();
    if trimmed.chars().count() <= max {
        return trimmed.to_string();
    }
    trimmed.chars().take(max).collect::<String>() + "…"
}
