use serde_json::json;
use std::collections::BTreeSet;
use rig::client::CompletionClient;
use rig::completion::CompletionRequestBuilder;
use rig::completion::{Message, ToolDefinition};
use rig::agent::run::{StreamedTurnAssembler, StreamedTurnEvent};
use rig::streaming::{StreamedAssistantContent, ToolCallDeltaContent};
use rig::message::{AssistantContent, ToolCall, ToolCallId, ToolFunction, ToolResultContent, UserContent};

fn assistant_tool_call_message(arguments: serde_json::Value) -> Message {
    let id = ToolCallId::new("call_args_1").expect("call_args_1");
    let call = ToolCall::new(id, ToolFunction::new("ssh_shell_exec".to_string(), arguments));
    Message::Assistant { id: None, content: vec![AssistantContent::ToolCall(call)] }
}

fn replayed_history(arguments: serde_json::Value) -> Vec<Message> {
    vec![
        Message::user("检查设备状态"),
        assistant_tool_call_message(arguments),
        Message::User { content: vec![UserContent::tool_result("call_args_1", "ssh_shell_exec", vec![ToolResultContent::text("工具 ssh_shell_exec 本轮不可用，未执行。")])] },
    ]
}

fn wire_arguments(message: Message) -> serde_json::Value {
    let wire: Vec<rig::providers::openai::completion::Message> = message.try_into().expect("转 openai wire 失败");
    let value = serde_json::to_value(&wire).expect("wire 序列化失败");
    value[0]["tool_calls"][0]["function"]["arguments"].clone()
}

#[test]
fn null_arguments_reach_openai_wire_as_empty_object() {
    assert_eq!(wire_arguments(assistant_tool_call_message(serde_json::Value::Null)), json!("{}"));
}

#[test]
fn object_arguments_preserved_on_openai_wire() {
    let arguments = wire_arguments(assistant_tool_call_message(json!({"command": "ls"})));
    let parsed: serde_json::Value = serde_json::from_str(arguments.as_str().expect("arguments 应为字符串")).expect("arguments 应为合法 JSON");
    assert_eq!(parsed, json!({"command": "ls"}));
}

#[test]
fn empty_object_arguments_reach_openai_wire_as_empty_object() {
    assert_eq!(wire_arguments(assistant_tool_call_message(json!({}))), json!("{}"));
}

#[test]
fn invalid_tool_call_diagnostic_arguments_are_object() {
    let allowed: BTreeSet<String> = ["chat_history".to_string()].into_iter().collect();
    let mut assembler = StreamedTurnAssembler::new(allowed.clone(), allowed);
    let item = StreamedAssistantContent::ToolCallDelta { internal_call_id: "ic_1".to_string(), content: ToolCallDeltaContent::Name("ssh_shell_exec".to_string()) };
    let events = assembler.ingest(&item).expect("ingest 失败");
    let StreamedTurnEvent::InvalidToolCall(invalid) = events.into_iter().next().expect("应产生事件") else {
        panic!("应产生 InvalidToolCall 事件");
    };
    assert_eq!(invalid.tool_call.function.arguments, json!({}));
}

#[test]
fn invalid_tool_call_with_partial_args_normalizes_to_object() {
    let allowed: BTreeSet<String> = ["chat_history".to_string()].into_iter().collect();
    let mut assembler = StreamedTurnAssembler::new(allowed.clone(), allowed);
    let args = StreamedAssistantContent::ToolCallDelta { internal_call_id: "ic_1".to_string(), content: ToolCallDeltaContent::Delta("{\"comm".to_string()) };
    assembler.ingest(&args).expect("ingest 失败");
    let name = StreamedAssistantContent::ToolCallDelta { internal_call_id: "ic_1".to_string(), content: ToolCallDeltaContent::Name("ssh_shell_exec".to_string()) };
    let events = assembler.ingest(&name).expect("ingest 失败");
    let StreamedTurnEvent::InvalidToolCall(invalid) = events.into_iter().next().expect("应产生事件") else {
        panic!("应产生 InvalidToolCall 事件");
    };
    assert_eq!(invalid.args.as_deref(), Some("{\"comm"));
    assert_eq!(invalid.tool_call.function.arguments, json!({}));
}

async fn send_replayed_history(arguments: serde_json::Value, label: &str) {
    let key = std::env::var("TERMINI_TEST_QWEN_KEY").expect("TERMINI_TEST_QWEN_KEY 未设置");
    let url = std::env::var("TERMINI_TEST_QWEN_URL").unwrap_or_else(|_| "https://dashscope.aliyuncs.com/compatible-mode/v1".to_string());
    let model_name = std::env::var("TERMINI_TEST_QWEN_MODEL").unwrap_or_else(|_| "qwen3.7-plus".to_string());
    let client = rig::providers::openai::CompletionsClient::builder().api_key(key.as_str()).base_url(url.as_str()).build().expect("构建 client 失败");
    let model = client.completion_model(model_name.as_str());
    let definition =
        ToolDefinition { name: "ssh_shell_exec".to_string(), description: "execute shell command".to_string(), parameters: json!({"type": "object", "properties": {"command": {"type": "string"}}}) };
    let response = CompletionRequestBuilder::new(model, "只回复两个字：好的").messages(replayed_history(arguments)).tool(definition).max_tokens(16).send().await;
    match response {
        Ok(_) => {}
        Err(error) => panic!("[{label}] 真实请求被拒: {error}"),
    }
}

#[tokio::test]
#[ignore]
async fn real_qwen_accepts_replayed_null_arguments() {
    send_replayed_history(serde_json::Value::Null, "arguments=Null(旧脏历史回放)").await;
}

#[tokio::test]
#[ignore]
async fn real_qwen_accepts_replayed_empty_object_arguments() {
    send_replayed_history(json!({}), "arguments={}(修复后常态)").await;
}
