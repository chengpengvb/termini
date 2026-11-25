use std::sync::Arc;
use serde_json::{json, Value};
use crate::module::ai::dto::AiProviderType;
use crate::model::resp::resp_ai_provider_info::{RespAiModelInfo, RespAiProviderInfo};

const VISION: bool = true;
const TEXT_ONLY: bool = false;

pub fn build_template_provider_info() -> Arc<Vec<RespAiProviderInfo>> {
    Arc::new(AiProviderType::ALL.iter().map(|provider| build_one(*provider)).collect())
}

pub fn resolve_no_thinking(provider: AiProviderType, model: &str) -> Option<Value> {
    template_models(provider).into_iter().find(|item| item.name == model).and_then(|item| item.no_thinking)
}

pub fn resolve_max_token(provider: AiProviderType, model: &str) -> Option<u32> {
    template_models(provider).into_iter().find(|item| item.name == model).map(|item| item.max_token)
}

fn build_one(provider: AiProviderType) -> RespAiProviderInfo {
    RespAiProviderInfo { provide_id: provider.as_i16(), name: provider.name().to_string(), models: template_models(provider), default_url: provider.default_url().map(str::to_string) }
}

fn template_models(provider: AiProviderType) -> Vec<RespAiModelInfo> {
    match provider {
        AiProviderType::OpenAi => vec![
            model("gpt-4o", 128000, VISION, None),
            model("gpt-4o-mini", 128000, VISION, None),
            model("gpt-4-turbo", 128000, VISION, None),
            model("gpt-4.1", 1048576, VISION, None),
            model("gpt-4.1-mini", 1048576, VISION, None),
            model("o3", 200000, VISION, Some(json!({ "reasoning_effort": "minimal" }))),
            model("o4-mini", 200000, VISION, Some(json!({ "reasoning_effort": "minimal" }))),
            model("gpt-5.4", 1000000, VISION, Some(json!({ "reasoning_effort": "minimal" }))),
            model("gpt-5.4-mini", 1000000, VISION, Some(json!({ "reasoning_effort": "minimal" }))),
        ],
        AiProviderType::Anthropic => vec![
            model("claude-opus-4-20250514", 200000, VISION, Some(thinking_disabled())),
            model("claude-sonnet-4-20250514", 200000, VISION, Some(thinking_disabled())),
            model("claude-sonnet-4-5-20250929", 200000, VISION, Some(thinking_disabled())),
            model("claude-haiku-4-5-20251001", 200000, VISION, Some(thinking_disabled())),
            model("claude-opus-4-5-20251101", 200000, VISION, Some(thinking_disabled())),
            model("claude-3-5-sonnet-20241022", 200000, VISION, None),
        ],
        AiProviderType::DeepSeek => vec![
            model("deepseek-chat", 1000000, TEXT_ONLY, None),
            model("deepseek-reasoner", 1000000, TEXT_ONLY, None),
            model("deepseek-v4-pro", 1000000, TEXT_ONLY, Some(thinking_disabled())),
            model("deepseek-v4-flash", 1000000, TEXT_ONLY, Some(thinking_disabled())),
        ],
        AiProviderType::Ollama => vec![model("llama3.3:70b", 128000, TEXT_ONLY, None), model("qwen3:32b", 128000, TEXT_ONLY, None), model("deepseek-r1:70b", 128000, TEXT_ONLY, None)],
        AiProviderType::OpenRouter => vec![
            model("anthropic/claude-sonnet-4", 200000, VISION, None),
            model("openai/gpt-4o", 128000, VISION, None),
            model("google/gemini-2.5-pro", 1048576, VISION, None),
            model("deepseek/deepseek-chat", 1000000, TEXT_ONLY, None),
            model("meta-llama/llama-3.3-70b-instruct", 131072, TEXT_ONLY, None),
        ],
        AiProviderType::Moonshot => vec![
            model("kimi-k3", 1000000, TEXT_ONLY, None),
            model("kimi-k2.6", 256000, TEXT_ONLY, Some(thinking_disabled())),
            model("kimi-k2.7-code", 256000, TEXT_ONLY, None),
            model("kimi-k2.7-code-highspeed", 256000, TEXT_ONLY, None),
        ],
        AiProviderType::Qwen => vec![
            model("qwen3.7-max", 1000000, TEXT_ONLY, Some(json!({ "enable_thinking": false }))),
            model("qwen3.7-plus", 1000000, TEXT_ONLY, Some(json!({ "enable_thinking": false }))),
            model("qwen3.7-max-preview", 1000000, TEXT_ONLY, Some(json!({ "enable_thinking": false }))),
        ],
        AiProviderType::Zhipu => vec![
            model("glm-5.2", 1000000, TEXT_ONLY, Some(thinking_disabled())),
            model("glm-5.1", 200000, TEXT_ONLY, Some(thinking_disabled())),
            model("glm-5", 200000, TEXT_ONLY, Some(thinking_disabled())),
            model("glm-5-turbo", 200000, TEXT_ONLY, Some(thinking_disabled())),
            model("glm-4.7", 200000, TEXT_ONLY, Some(thinking_disabled())),
            model("glm-4.7-flash", 200000, TEXT_ONLY, Some(thinking_disabled())),
            model("glm-5v-turbo", 200000, VISION, Some(thinking_disabled())),
        ],
        // grok-4.6/4.5 无法关闭思考, reasoning_effort=low 是最快档; 4.20-non-reasoning 天然不思考
        AiProviderType::Xai => vec![
            model("grok-4.6", 500000, VISION, Some(json!({ "reasoning_effort": "low" }))),
            model("grok-4.5", 500000, VISION, Some(json!({ "reasoning_effort": "low" }))),
            model("grok-4.3", 1000000, VISION, None),
            model("grok-4.20-0309-non-reasoning", 1000000, VISION, None),
            model("grok-4.20-0309-reasoning", 1000000, VISION, None),
            model("grok-build-0.1", 256000, TEXT_ONLY, None),
        ],
        AiProviderType::Custom => Vec::new(),
    }
}

fn thinking_disabled() -> Value {
    json!({ "thinking": { "type": "disabled" } })
}

fn model(name: &str, max_token: u32, vision: bool, no_thinking: Option<Value>) -> RespAiModelInfo {
    RespAiModelInfo { name: name.to_string(), max_token, vision, no_thinking }
}
