use std::sync::Arc;
use std::time::Duration;
use summer::tracing::warn;
use anyhow::{anyhow, Result};
use rig::completion::Message;
use rig::agent::StreamingError;
use crate::module::ai::agent::AgentRole;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::stream::stream_output::StreamOutput;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::module::ai::stream::stream_runner::{AiStreamRunner, StreamErrorEmitted};

const COMPLETION_RETRY_DELAY_SECS: [u64; 2] = [2, 5];
pub(in crate::module::ai::stream) const COMPLETION_RETRY_MAX: usize = 2;

impl AiStreamRunner {
    pub(in crate::module::ai::stream) async fn retry_completion<O>(&self, context: Arc<AiRunContext>, role: AgentRole, error: StreamingError, retries_left: usize) -> Result<O>
    where
        O: StreamOutput,
    {
        let mut history = context.memory.lock().clone();
        let Some(Message::User { .. }) = history.last() else {
            warn!("[ai] 记忆末尾不是用户消息, 放弃重试 req_id={} run_id={}", context.req_id, context.run_id);
            if self.handle_error(&context, role, error).await {
                return Err(anyhow!(StreamErrorEmitted));
            }
            return Ok(O::from_stream_text(""));
        };
        let Some(prompt_message) = history.pop() else {
            return Ok(O::from_stream_text(""));
        };
        let attempt = COMPLETION_RETRY_MAX - retries_left;
        let delay = Duration::from_secs(COMPLETION_RETRY_DELAY_SECS[attempt.min(COMPLETION_RETRY_DELAY_SECS.len() - 1)]);
        warn!(
            "[ai] 补全请求瞬时失败, {}s 后原地重发 attempt={}/{} req_id={} run_id={} history_msgs={} error={}",
            delay.as_secs(),
            attempt + 1,
            COMPLETION_RETRY_MAX,
            context.req_id,
            context.run_id,
            history.len(),
            error
        );
        ProvideFactory::reset_http_client();
        tokio::select! {
            biased;
            _ = context.cancel_token.cancelled() => return Ok(O::from_stream_text("")),
            _ = tokio::time::sleep(delay) => {}
        }
        let context = Arc::new(context.with_rebuilt_agent()?);
        if context.emit.status {
            self.emit_retry_status(&context, attempt + 1, COMPLETION_RETRY_MAX);
        }
        Box::pin(self.run_completion(&context.agent, Arc::clone(&context), role, history.clone(), prompt_message.clone(), retries_left - 1)).await
    }
}
