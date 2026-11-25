use rig::completion::Usage;

#[derive(Debug, Clone, Copy, Default)]
pub struct RunTrace {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub context_tokens: u64,
    pub completion_calls: usize,
}

impl RunTrace {
    pub fn absorb(&mut self, usage: &Usage, completion_calls: usize) {
        self.input_tokens += usage.input_tokens;
        self.output_tokens += usage.output_tokens;
        self.completion_calls += completion_calls.max(1);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunEndState {
    Done,
    Cancelled,
    Failed,
}

impl RunEndState {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunEndState::Done => "done",
            RunEndState::Failed => "failed",
            RunEndState::Cancelled => "cancelled",
        }
    }
}
