use tokio::task::JoinHandle;
use rig::completion::Message;
use crate::model::ai_provide::AiProvide;
use crate::module::ai::shared::mem::{budget, synthetic};
use crate::module::ai::shared::mem::aging::{age_tool_results, AGE_KEEP_RECENT_TURNS};

pub struct PrefetchFold {
    pub from: usize,
    pub end: usize,
    pub handle: JoinHandle<Option<String>>,
}

#[derive(Default)]
pub struct FoldState {
    pub summary: String,
    pub folded: usize,
    pub model_failures: usize,
    pub prefetch: Option<PrefetchFold>,
}

impl FoldState {
    pub fn new(summary: String) -> Self {
        Self { summary, folded: 0, model_failures: 0, prefetch: None }
    }
}

pub struct Compacted {
    pub messages: Vec<Message>,
    pub summary: String,
    pub dropped: usize,
    pub aged: usize,
}

pub struct FoldPlan {
    pub limit: usize,
    pub tokens: usize,
    pub aged: usize,
    pub provide: AiProvide,
    pub pinned: Vec<Message>,
    pub messages: Vec<Message>,
    pub keep_budget: usize,
    pub pinned_tokens: usize,
}

impl FoldPlan {
    pub fn build(token_limit: usize, provide: AiProvide, addon: &[Message], messages: &[Message], reserved: usize, summary: &str) -> Self {
        let limit = budget::fold_limit(token_limit);
        let mut aged_messages = messages.to_vec();
        let aged = age_tool_results(&mut aged_messages, AGE_KEEP_RECENT_TURNS);
        let pinned = budget::pinned_messages(&aged_messages, addon);
        let pinned_tokens = budget::estimate(&pinned);
        let summary_tokens = synthetic::summary_tokens(summary);
        let tokens = reserved + summary_tokens + budget::estimate(&aged_messages);
        let keep_budget = budget::keep_budget(limit, reserved + summary_tokens + pinned_tokens);
        Self { limit, tokens, aged, provide, pinned, messages: aged_messages, keep_budget, pinned_tokens }
    }

    pub fn over_high(&self) -> bool {
        self.tokens > self.limit * budget::FOLD_HIGH_PERCENT / 100
    }

    pub fn over_low(&self) -> bool {
        self.tokens > self.limit * budget::FOLD_LOW_PERCENT / 100
    }

    pub fn intact(&self, summary: &str) -> Compacted {
        Compacted { messages: budget::compose(&self.pinned, self.messages.clone()), summary: summary.to_string(), dropped: 0, aged: self.aged }
    }

    pub fn to_fold(&self) -> Vec<Message> {
        budget::split_tail(&self.messages, self.keep_budget)
    }
}
