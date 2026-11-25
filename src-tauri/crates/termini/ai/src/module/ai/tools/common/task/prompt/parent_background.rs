use rig::completion::Message;
use rig::message::AssistantContent;
use crate::module::ai::tools::common::task::rule::scope;
use crate::module::ai::shared::chat_item_format::truncate_chars;

const MAX_ITEMS: usize = 30;

pub fn parent_background(user_request: &str, summary: &str, messages: &[Message]) -> String {
    let mut read_paths: Vec<String> = Vec::new();
    let mut searches: Vec<String> = Vec::new();
    for message in messages {
        let Message::Assistant { content, .. } = message else {
            continue;
        };
        for part in content.iter() {
            let AssistantContent::ToolCall(call) = part else {
                continue;
            };
            let args = &call.function.arguments;
            let root = args.get("root").and_then(|value| value.as_str());
            match call.function.name.as_str() {
                "file_read" => {
                    if let Some(path) = args.get("path").and_then(|value| value.as_str()) {
                        push_unique(&mut read_paths, scope::join(root, path));
                    }
                    for item in args.get("files").and_then(|value| value.as_array()).into_iter().flatten() {
                        if let Some(path) = item.get("path").and_then(|value| value.as_str()) {
                            push_unique(&mut read_paths, scope::join(root, path));
                        }
                    }
                }
                "code_semantic" | "code_search" => {
                    let text = args
                        .get("query")
                        .and_then(|value| value.as_str())
                        .map(|value| value.to_string())
                        .or_else(|| args.get("queries").and_then(|value| value.as_array()).map(|list| list.iter().filter_map(|item| item.as_str()).collect::<Vec<_>>().join(" | ")));
                    if let Some(text) = text {
                        push_unique(&mut searches, format!("{} {}", call.function.name, text));
                    }
                }
                _ => {}
            }
        }
    }

    let mut out = String::new();
    let user_request = user_request.trim();
    if !user_request.is_empty() {
        out.push_str(&format!("用户的原始问题: {}\n", truncate_chars(user_request, 300)));
    }
    let summary = summary.trim();
    if !summary.is_empty() {
        out.push_str(&format!("主代理此前的上下文摘要: {}\n", truncate_chars(summary, 800)));
    }
    if !read_paths.is_empty() {
        out.push_str(&format!("主代理已经读过的文件({} 个, 说明它们和问题相关):\n", read_paths.len()));
        for path in read_paths.iter().take(MAX_ITEMS) {
            out.push_str(&format!("- {}\n", path));
        }
    }
    if !searches.is_empty() {
        out.push_str("主代理已经做过的检索(不必重复):\n");
        for item in searches.iter().take(MAX_ITEMS) {
            out.push_str(&format!("- {}\n", truncate_chars(item, 120)));
        }
    }
    out
}

fn push_unique(list: &mut Vec<String>, item: String) {
    if !list.iter().any(|seen| seen.eq_ignore_ascii_case(&item)) {
        list.push(item);
    }
}
