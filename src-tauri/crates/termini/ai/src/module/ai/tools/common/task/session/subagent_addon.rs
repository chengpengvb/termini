use rig::agent::HookStack;
use crate::module::ai::tools::common::task::hook::scope_guard::ScopeGuardHook;
use crate::module::ai::tools::common::task::session::task_registry::TaskEntry;
use crate::module::ai::tools::common::task::hook::search_fanout::SearchFanoutHook;
use crate::module::ai::tools::common::task::hook::task_progress::TaskProgressHook;
use crate::module::ai::tools::common::task::hook::search_nomatch::SearchNoMatchHook;

pub struct SubagentRunAddon {
    scope: Vec<String>,
    fanout: SearchFanoutHook,
    nomatch: SearchNoMatchHook,
    progress: TaskProgressHook,
}

impl SubagentRunAddon {
    pub fn new(scope: Vec<String>, task_entry: TaskEntry) -> Self {
        Self { scope, fanout: SearchFanoutHook::default(), nomatch: SearchNoMatchHook::default(), progress: TaskProgressHook { task_entry } }
    }

    pub fn install(&self, stack: &mut HookStack) {
        stack.push(ScopeGuardHook::new(self.scope.clone()));
        stack.push(self.fanout.clone());
        stack.push(self.nomatch.clone());
        stack.push(self.progress.clone());
    }
}
