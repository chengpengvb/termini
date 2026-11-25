use serde_json::json;
use crate::util::id_factory::uuid;
use crate::util::time_util::now_millis;
use crate::model::ai_chat_item::AiChatItem;
use crate::model::resp::event::resp_event_agent::RespEventAgent;
use crate::module::ai::shared::chat_item_format::{render_chat_history, ChatHistoryScope};

fn user_item(message: &str) -> AiChatItem {
    AiChatItem { id: uuid(), chat_id: "c1".to_string(), r#type: RespEventAgent::ON_USER, timestamp: now_millis(), context: json!({ "message": message }), pid: None }
}

fn assistant_item(delta: &str) -> AiChatItem {
    AiChatItem { id: uuid(), chat_id: "c1".to_string(), r#type: RespEventAgent::ON_TEXT_FINAL, timestamp: now_millis(), context: json!({ "delta": delta }), pid: None }
}

#[test]
fn chat_history_user_scope_keeps_user_only_newest_first_window() {
    let items = vec![user_item("修复 nginx 502"), assistant_item("正在检查"), user_item("继续"), assistant_item("看日志"), user_item("然后重启")];
    let rendered = render_chat_history(&items, ChatHistoryScope::User, 10, None);
    assert!(rendered.contains("[USER] 修复 nginx 502"));
    assert!(rendered.contains("[USER] 继续"));
    assert!(rendered.contains("[USER] 然后重启"));
    assert!(!rendered.contains("[ASSISTANT]"));
}

fn tool_call_item(tool_id: &str, args: serde_json::Value) -> AiChatItem {
    AiChatItem { id: uuid(), chat_id: "c1".to_string(), r#type: RespEventAgent::ON_TOOL_CALL, timestamp: now_millis(), context: json!({ "toolId": tool_id, "args": args }), pid: None }
}

#[test]
fn chat_history_search_finds_trimmed_tool_args_with_context() {
    let mut items = vec![user_item("部署袋鼠保")];
    for index in 0..20 {
        items.push(assistant_item(&format!("步骤 {index}")));
    }
    items.push(tool_call_item("ssh_shell_exec", json!({ "device_id": "dev-1", "command": "cd /home/soft/dskb/biz && ./run_app.sh" })));
    items.push(assistant_item("已启动"));
    for index in 0..20 {
        items.push(assistant_item(&format!("后续 {index}")));
    }

    let rendered = render_chat_history(&items, ChatHistoryScope::Search, 8, Some("run_app.sh"));
    assert!(rendered.contains("[TOOL_CALL] ssh_shell_exec"));
    assert!(rendered.contains("/home/soft/dskb/biz"));
    assert!(rendered.contains("已启动"));
}

#[test]
fn chat_history_search_requires_query() {
    let items = vec![user_item("部署")];
    let rendered = render_chat_history(&items, ChatHistoryScope::Search, 8, None);
    assert!(rendered.contains("需要 query"));
}

#[test]
fn chat_history_query_filters_user_lines() {
    let items = vec![user_item("安装 docker"), user_item("继续"), user_item("排查磁盘")];
    let rendered = render_chat_history(&items, ChatHistoryScope::User, 10, Some("docker"));
    assert!(rendered.contains("安装 docker"));
    assert!(!rendered.contains("排查磁盘"));
}
