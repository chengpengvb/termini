use anyhow::Result;
use std::sync::Arc;
use summer::plugin::Service;
use termini_macros::base_app;
use crate::util::id_factory::uuid;
use crate::base::base_app::BaseApp;
use crate::util::time_util::now_millis;
use crate::module::ai::agent::AgentRole;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::derived::DerivedStore;
use crate::module::ai::shared::ChatFileStore;
use crate::module::ai::shared::MemoryCompactor;
use crate::module::ai::shared::ToolCallDonePush;
use crate::service::service_agent::AgentService;
use crate::module::ai::shared::GeneratedImagePush;
use crate::module::ai::shared::ToolActivationStore;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::stream::stream_output::StreamOutput;

pub(in crate::module::ai::stream) const TOOL_CALL_CANCELLED_RESULT: &str = "exit_code=cancelled";
pub(in crate::module::ai::stream) const STREAM_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

#[derive(Debug)]
pub struct StreamIdleTimeout(pub u64);

impl std::fmt::Display for StreamIdleTimeout {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no output from model for {}s", self.0)
    }
}

impl std::error::Error for StreamIdleTimeout {}

#[derive(Debug)]
pub struct StreamErrorEmitted;

impl StreamErrorEmitted {
    pub fn displayed(error: &anyhow::Error) -> bool {
        error.downcast_ref::<StreamErrorEmitted>().is_some()
    }
}

impl std::fmt::Display for StreamErrorEmitted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "stream error already emitted")
    }
}

impl std::error::Error for StreamErrorEmitted {}

pub(crate) struct OpenSegment {
    pub id: String,
    pub timestamp: i64,
    pub content: String,
}

impl OpenSegment {
    pub(crate) fn open() -> Self {
        Self { id: uuid(), content: String::new(), timestamp: now_millis() }
    }
}

#[base_app(sort = 771)]
#[derive(Clone, Service)]
pub struct AiStreamRunner {
    #[inject(component)]
    pub derived_store: DerivedStore,

    #[inject(component)]
    pub agent_service: AgentService,

    #[inject(component)]
    pub chat_file_store: ChatFileStore,

    #[inject(component)]
    pub memory_compactor: MemoryCompactor,

    #[inject(component)]
    pub tool_call_done_push: ToolCallDonePush,

    #[inject(component)]
    pub generated_image_push: GeneratedImagePush,

    #[inject(component)]
    pub tool_activation_store: ToolActivationStore,

    #[inject(component)]
    pub chat_session_cache_manager: ChatSessionCacheManager,
}

impl BaseApp for AiStreamRunner {
    fn name(&self) -> &'static str {
        "AiStreamRunner"
    }
}

impl AiStreamRunner {
    pub async fn run_stream<O: StreamOutput>(&self, context: Arc<AiRunContext>, role: AgentRole) -> Result<O> {
        self.consume_stream(&context.agent, Arc::clone(&context), role).await
    }
}
