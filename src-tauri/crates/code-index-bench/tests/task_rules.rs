extern crate code_index_bench as termini_ai;

#[path = "../../termini/ai/tests/web_html_text.rs"]
mod web_html_text;
#[path = "../../termini/ai/tests/task_scope_rule.rs"]
mod task_scope_rule;
#[path = "../../termini/ai/tests/task_read_dedup_rule.rs"]
mod task_read_dedup_rule;
#[path = "../../termini/ai/tests/task_search_fanout_rule.rs"]
mod task_search_fanout_rule;
#[path = "../../termini/ai/tests/task_search_nomatch_rule.rs"]
mod task_search_nomatch_rule;
#[path = "../../termini/ai/tests/task_shell_readonly_rule.rs"]
mod task_shell_readonly_rule;