use std::sync::Arc;
use parking_lot::RwLock;
use rig::tool::DynamicTool;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use termini_macros::base_app;
use rig::prelude::AgentClientExt;
use crate::base::base_app::BaseApp;
use rig::http_client::ReqwestClient;
use crate::model::ai_provide::AiProvide;
use rig::tool::server::ToolServerHandle;
use crate::module::ai::dto::AiProviderType;
use crate::module::ai::provide::provide_template;
use crate::module::ai::provide::ai_agent::AiAgent;
use rig::providers::{anthropic, deepseek, moonshot, ollama, openai, openrouter};

const PARAM_ID: &str = "id";

const MAX_OUTPUT_TOKENS: u64 = 32768;

const READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
const POOL_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

static HTTP_CLIENT: RwLock<Option<ReqwestClient>> = RwLock::new(None);

macro_rules! build_provider_client {
    ($client:path, $key:expr, $url:expr, $provider_type:expr, $http_client:expr) => {{
        let mut builder = <$client>::builder().api_key($key).http_client($http_client);
        if !$url.is_empty() {
            builder = builder.base_url($url);
        }
        builder.build().map_err(|err| anyhow!("failed to build {:?} client: {}", $provider_type, err))?
    }};
}

macro_rules! match_ai_provider {
    ($provider_type:expr, $build:ident) => {
        match $provider_type {
            AiProviderType::Ollama => $build!(ollama::Client),
            AiProviderType::DeepSeek => $build!(deepseek::Client),
            AiProviderType::Moonshot => $build!(moonshot::Client),
            AiProviderType::Anthropic => $build!(anthropic::Client),
            AiProviderType::OpenRouter => $build!(openrouter::Client),
            AiProviderType::OpenAi | AiProviderType::Custom | AiProviderType::Qwen | AiProviderType::Zhipu | AiProviderType::Xai => $build!(openai::CompletionsClient),
        }
    };
}

#[base_app(sort = 770)]
#[derive(Clone, Service)]
pub struct ProvideFactory {}

impl BaseApp for ProvideFactory {
    fn name(&self) -> &'static str {
        "ProvideFactory"
    }
}

impl ProvideFactory {
    pub fn build_agent_with_thinking(&self, ai_provide: &AiProvide, preamble: &str, tools: Vec<DynamicTool>) -> Result<AiAgent> {
        self.finish_agent(ai_provide, preamble, tools, None, Self::id_params(ai_provide))
    }

    pub fn build_agent_with_level(&self, ai_provide: &AiProvide, preamble: &str, tools: Vec<DynamicTool>, level: Option<&str>) -> Result<AiAgent> {
        self.finish_agent(ai_provide, preamble, tools, None, Self::level_params(ai_provide, level))
    }

    pub fn build_agent_with_level_handle(&self, ai_provide: &AiProvide, preamble: &str, handle: ToolServerHandle, level: Option<&str>) -> Result<AiAgent> {
        self.finish_agent(ai_provide, preamble, Vec::new(), Some(handle), Self::level_params(ai_provide, level))
    }

    pub fn build_agent_without_thinking(&self, ai_provide: &AiProvide, preamble: &str, tools: Vec<DynamicTool>) -> Result<AiAgent> {
        let provider_code = ai_provide.provider.ok_or_else(|| anyhow!("ai_provide.provider is not set"))?;
        let provider_type = AiProviderType::from_i16(provider_code).ok_or_else(|| anyhow!("unknown provider code: {}", provider_code))?;
        let mut extra_params = Self::id_params(ai_provide);
        let cloud_params = Self::cloud_no_thinking_params(ai_provide);
        if !cloud_params.is_empty() {
            summer::tracing::info!("[ai] 使用云端下发的关闭思考参数 provider={:?} model={} params={}", provider_type, ai_provide.model, serde_json::Value::Object(cloud_params.clone()));
            extra_params.extend(cloud_params);
        } else {
            match provide_template::resolve_no_thinking(provider_type, ai_provide.model.as_str()) {
                Some(serde_json::Value::Object(map)) => {
                    summer::tracing::info!("[ai] 使用本地模板的关闭思考参数 provider={:?} model={} params={}", provider_type, ai_provide.model, serde_json::Value::Object(map.clone()));
                    extra_params.extend(map);
                }
                _ => {
                    summer::tracing::warn!("[ai] 云端与本地模板均无关闭思考参数, 该模型仍会思考 provider={:?} model={}", provider_type, ai_provide.model);
                }
            }
        }
        self.finish_agent(ai_provide, preamble, tools, None, extra_params)
    }

    fn level_params(ai_provide: &AiProvide, level: Option<&str>) -> serde_json::Map<String, serde_json::Value> {
        let mut extra_params = Self::id_params(ai_provide);
        if let Some(params) = ai_provide.thinking_level_params(level) {
            summer::tracing::info!("[ai] model={} level={} params={}", ai_provide.model, level.unwrap_or("default"), serde_json::Value::Object(params.clone()));
            extra_params.extend(params);
        }
        extra_params
    }

    fn cloud_no_thinking_params(ai_provide: &AiProvide) -> serde_json::Map<String, serde_json::Value> {
        let mut out = serde_json::Map::new();
        let Some(serde_json::Value::Object(map)) = &ai_provide.params else {
            return out;
        };
        for (key, value) in map {
            if key != PARAM_ID {
                out.insert(key.clone(), value.clone());
            }
        }
        out
    }

    pub fn id_params(ai_provide: &AiProvide) -> serde_json::Map<String, serde_json::Value> {
        let mut extra_params = serde_json::Map::new();
        let Some(serde_json::Value::Object(map)) = &ai_provide.params else {
            return extra_params;
        };
        let Some(id) = map.get(PARAM_ID) else {
            return extra_params;
        };
        extra_params.insert(PARAM_ID.to_string(), id.clone());
        extra_params
    }

    fn resolve_max_output(ai_provide: &AiProvide) -> u64 {
        let Some(context_limit) = Self::resolve_context_limit(ai_provide) else {
            return MAX_OUTPUT_TOKENS;
        };
        MAX_OUTPUT_TOKENS.min(context_limit / 2)
    }

    fn resolve_context_limit(ai_provide: &AiProvide) -> Option<u64> {
        if let Some(value) = ai_provide.max_token.filter(|value| *value > 0) {
            return Some(value as u64);
        }
        let provider_code = ai_provide.provider?;
        let provider_type = AiProviderType::from_i16(provider_code)?;
        provide_template::resolve_max_token(provider_type, &ai_provide.model).map(|value| value as u64)
    }

    fn finish_agent(
        &self,
        ai_provide: &AiProvide,
        preamble: &str,
        tools: Vec<DynamicTool>,
        tool_handle: Option<ToolServerHandle>,
        extra_params: serde_json::Map<String, serde_json::Value>,
    ) -> Result<AiAgent> {
        let model = ai_provide.model.as_str();
        let url = ai_provide.url.trim().to_string();
        let max_output = Self::resolve_max_output(ai_provide);
        let key = ai_provide.key.clone().unwrap_or_default();
        let provider_code = ai_provide.provider.ok_or_else(|| anyhow!("ai_provide.provider is not set"))?;
        let provider_type = AiProviderType::from_i16(provider_code).ok_or_else(|| anyhow!("unknown provider code: {}", provider_code))?;

        let http_client = Self::http_client()?;

        macro_rules! build_client_agent {
            ($client:path) => {{
                let client = build_provider_client!($client, key.as_str(), url.as_str(), provider_type, http_client.clone());
                let mut agent_builder = client.agent(model).max_tokens(max_output);
                if !preamble.is_empty() {
                    agent_builder = agent_builder.preamble(preamble);
                }
                if !extra_params.is_empty() {
                    agent_builder = agent_builder.additional_params(serde_json::Value::Object(extra_params.clone()));
                }
                if let Some(handle) = tool_handle.clone() {
                    agent_builder.tool_server_handle(handle).build()
                } else if tools.is_empty() {
                    agent_builder.build()
                } else {
                    agent_builder.dynamic_tools(tools.clone()).build()
                }
            }};
        }

        Ok(match_ai_provider!(provider_type, build_client_agent))
    }

    pub fn reset_http_client() {
        HTTP_CLIENT.write().take();
    }

    fn rustls_client_config() -> Result<rustls::ClientConfig> {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|err| anyhow!("failed to init rustls versions: {}", err))?
            .with_root_certificates(roots)
            .with_no_client_auth();
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        Ok(config)
    }

    pub fn http_client() -> Result<ReqwestClient> {
        if let Some(client) = HTTP_CLIENT.read().clone() {
            return Ok(client);
        }
        let client = ReqwestClient::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .pool_idle_timeout(POOL_IDLE_TIMEOUT)
            .http1_only()
            .local_address(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED))
            .tls_backend_preconfigured(Self::rustls_client_config()?)
            .build()
            .map_err(|err| anyhow!("failed to build provider http client: {}", err))?;
        Ok(HTTP_CLIENT.write().get_or_insert(client).clone())
    }
}
