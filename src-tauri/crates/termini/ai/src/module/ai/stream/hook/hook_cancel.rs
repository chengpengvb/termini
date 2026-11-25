use rig::agent::{
    AgentHook, CompletionCallAction, CompletionCallEvent, CompletionResponseEvent, HookContext, InvalidToolCallAction, InvalidToolCallContext, ModelSelection, ModelSelectionAction, ModelTurnAction,
    ModelTurnFinished, ObservationAction, ReasoningDelta, StreamResponseFinish, TextDelta, ToolCall, ToolCallAction, ToolCallDelta, ToolResultAction, ToolResultEvent,
};
use tokio_util::sync::CancellationToken;

pub struct CancelHook {
    pub token: CancellationToken,
}

impl CancelHook {
    fn cancelled(&self) -> bool {
        self.token.is_cancelled()
    }
}

impl AgentHook for CancelHook {
    fn on_model_select(&self, _ctx: &HookContext, _event: ModelSelection<'_>) -> ModelSelectionAction {
        if self.cancelled() {
            return ModelSelectionAction::stop("user cancel");
        }
        ModelSelectionAction::Continue
    }

    async fn on_completion_call(&self, _ctx: &HookContext, _event: CompletionCallEvent<'_>) -> CompletionCallAction {
        if self.cancelled() {
            return CompletionCallAction::stop("user cancel");
        }
        CompletionCallAction::Continue
    }

    async fn on_completion_response(&self, _ctx: &HookContext, _event: CompletionResponseEvent<'_>) -> ObservationAction {
        if self.cancelled() {
            return ObservationAction::stop("user cancel");
        }
        ObservationAction::Continue
    }

    async fn on_model_turn_finished(&self, _ctx: &HookContext, _event: ModelTurnFinished<'_>) -> ModelTurnAction {
        if self.cancelled() {
            return ModelTurnAction::stop("user cancel");
        }
        ModelTurnAction::Continue
    }

    async fn on_invalid_tool_call(&self, _ctx: &HookContext, _event: &InvalidToolCallContext) -> Option<InvalidToolCallAction> {
        if self.cancelled() {
            return Some(InvalidToolCallAction::stop("user cancel"));
        }
        None
    }

    async fn on_tool_call(&self, _ctx: &HookContext, _event: ToolCall<'_>) -> ToolCallAction {
        if self.cancelled() {
            return ToolCallAction::stop("user cancel");
        }
        ToolCallAction::Run
    }

    async fn on_tool_result(&self, _ctx: &HookContext, _event: ToolResultEvent<'_>) -> ToolResultAction {
        if self.cancelled() {
            return ToolResultAction::stop("user cancel");
        }
        ToolResultAction::Keep
    }

    async fn on_text_delta(&self, _ctx: &HookContext, _event: TextDelta<'_>) -> ObservationAction {
        if self.cancelled() {
            return ObservationAction::stop("user cancel");
        }
        ObservationAction::Continue
    }

    async fn on_reasoning_delta(&self, _ctx: &HookContext, _event: ReasoningDelta<'_>) -> ObservationAction {
        if self.cancelled() {
            return ObservationAction::stop("user cancel");
        }
        ObservationAction::Continue
    }

    async fn on_tool_call_delta(&self, _ctx: &HookContext, _event: ToolCallDelta<'_>) -> ObservationAction {
        if self.cancelled() {
            return ObservationAction::stop("user cancel");
        }
        ObservationAction::Continue
    }

    async fn on_stream_response_finish(&self, _ctx: &HookContext, _event: StreamResponseFinish<'_>) -> ObservationAction {
        if self.cancelled() {
            return ObservationAction::stop("user cancel");
        }
        ObservationAction::Continue
    }
}
