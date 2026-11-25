use anyhow::Context;
use summer::plugin::Service;
use async_trait::async_trait;
use std::collections::BTreeMap;
use crate::model::ai_mcp_server;
use crate::dao::dao_base::BaseDao;
use crate::base::base_app::BaseApp;
use crate::model::base_target::BaseTarget;
use crate::model::dto::shared_ptr::Shared;
use crate::model::ai_mcp_server::AiMcpServer;
use termini_macros::{base_app, base_service};
use crate::service::service_base::BaseService;
use crate::dao::dao_ai_mcp_server::AiMcpServerDao;
use crate::service::service_base_api::BaseApiService;
use crate::model::req::req_mcp_market::ReqMcpMarketInstall;
use crate::model::resp::resp_model::{BizError, ResultCode};
use crate::module::ai::mcp::mcp_client_pool::McpClientPool;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
use crate::module::ai::mcp::mcp_server_runtime::McpServerRuntime;
use crate::module::ai::mcp::dto::mcp_tool_snapshot::McpToolSnapshot;
use crate::module::ai::mcp::mcp_command_resolver::McpCommandResolver;
use crate::module::ai::mcp::dto::mcp_prompt_snapshot::McpPromptSnapshot;
use crate::model::resp::resp_ai_mcp_server::{RespAiMcpServerRuntime, RespAiMcpServerTools};
use crate::module::ai::mcp::dto::mcp_transport_kind::{MCP_TRANSPORT_HTTP, MCP_TRANSPORT_STDIO};
use crate::model::server::mcp_market::{McpMarketEnvKey, McpMarketItem, MCP_MARKET_RUNTIME_BINARY, MCP_MARKET_RUNTIME_HTTP, MCP_MARKET_RUNTIME_NODE, MCP_MARKET_RUNTIME_PYTHON};

#[base_app(sort = 201)]
#[base_service(ai_mcp_server::Entity, dao = "ai_mcp_server_dao", sync = 13)]
#[derive(Clone, Service)]
pub struct AiMcpServerService {
    #[inject(component)]
    mcp_client_pool: McpClientPool,

    #[inject(component)]
    ai_mcp_server_dao: AiMcpServerDao,

    #[inject(component)]
    mcp_command_resolver: McpCommandResolver,
}

#[async_trait]
impl BaseService for AiMcpServerService {
    type Entity = ai_mcp_server::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.ai_mcp_server_dao.clone()))
    }

    async fn insert(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let list = self.find_all().await?;
        if let Some(found) = list.into_iter().find(|item| item.read().name == entity.name) {
            return Err(BizError::duplicate(found.read().id.clone()).into());
        }
        self.get_dao()?.insert(entity).await
    }

    async fn update(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let list = self.find_all().await?;
        for found in &list {
            let row = found.read();
            if row.id != entity.id && row.name == entity.name {
                return Err(BizError::duplicate(row.id.clone()).into());
            }
        }
        self.get_dao()?.update(entity).await
    }
}

impl BaseApp for AiMcpServerService {
    fn name(&self) -> &'static str {
        "AiMcpServerService"
    }
}

impl BaseApiService for AiMcpServerService {}

impl AiMcpServerService {
    pub async fn create_server(&self, mut server: AiMcpServer) -> anyhow::Result<Shared<AiMcpServer>> {
        self.normalize_server(&mut server)?;
        let saved = self.ai_mcp_server_dao.insert(server).await?;
        self.mcp_client_pool.sync_from_db().await;
        Ok(saved)
    }

    pub async fn update_server(&self, mut server: AiMcpServer) -> anyhow::Result<Shared<AiMcpServer>> {
        self.normalize_server(&mut server)?;
        let saved = self.ai_mcp_server_dao.update(server).await?;
        self.mcp_client_pool.sync_from_db().await;
        Ok(saved)
    }

    pub async fn delete_server(&self, id: String) -> anyhow::Result<()> {
        self.ai_mcp_server_dao.delete_by_id(id.clone()).await?;
        self.mcp_client_pool.stop_server(&id);
        self.mcp_client_pool.sync_from_db().await;
        Ok(())
    }

    pub async fn list_runtime(&self) -> anyhow::Result<Vec<RespAiMcpServerRuntime>> {
        let list = self.ai_mcp_server_dao.find_all().await?;
        let mut out = Vec::with_capacity(list.len());
        for item in list {
            let server = item.read().clone();
            let (connected, tool_count, last_error) = self.mcp_client_pool.runtime_status(&server);
            out.push(RespAiMcpServerRuntime { server, connected, tool_count, last_error });
        }
        out.sort_by_key(|item| item.server.sort);
        Ok(out)
    }

    pub async fn reconnect(&self, id: String) -> anyhow::Result<RespAiMcpServerRuntime> {
        self.mcp_client_pool.reconnect(&id).await?;
        let server = self.ai_mcp_server_dao.find_by_id(id).await?.ok_or_else(|| anyhow::Error::from(ResultCode::McpServerNotFound))?.read().clone();
        let (connected, tool_count, last_error) = self.mcp_client_pool.runtime_status(&server);
        Ok(RespAiMcpServerRuntime { server, connected, tool_count, last_error })
    }

    pub fn list_tools(&self, id: String) -> anyhow::Result<RespAiMcpServerTools> {
        let tools = self.mcp_client_pool.list_snapshots(&id)?;
        Ok(RespAiMcpServerTools { tools })
    }

    pub fn list_prompts(&self) -> anyhow::Result<Vec<McpPromptSnapshot>> {
        Ok(self.mcp_client_pool.list_prompts())
    }

    pub fn list_connected_tools(&self) -> anyhow::Result<Vec<McpToolSnapshot>> {
        Ok(self.mcp_client_pool.list_tools())
    }

    pub async fn probe(&self, mut server: AiMcpServer) -> anyhow::Result<Vec<McpToolSnapshot>> {
        self.normalize_server(&mut server)?;
        let resolved = self.mcp_command_resolver.resolve(&server).await?;
        McpServerRuntime::probe(&resolved).await
    }

    pub async fn fetch_market(&self) -> anyhow::Result<Vec<McpMarketItem>> {
        Self::api_data(Self::api_get("/api/mcpMarketApi/list")).await
    }

    pub async fn install_from_market(&self, req: ReqMcpMarketInstall) -> anyhow::Result<Shared<AiMcpServer>> {
        let market = self.fetch_market().await?;
        let item = market.into_iter().find(|item| item.id == req.market_id).ok_or_else(|| anyhow::anyhow!("mcp market item not found: {}", req.market_id))?;
        let existing = self.ai_mcp_server_dao.find_all().await?;
        if existing.iter().any(|server| server.read().name == item.name) {
            return Err(anyhow::anyhow!("MCP server already installed: {}", item.name));
        }
        let sort = existing.iter().map(|server| server.read().sort).max().unwrap_or(0) + 1;
        let server = Self::build_from_market(&item, req.env, sort)?;
        self.create_server(server).await
    }

    fn build_from_market(item: &McpMarketItem, env: BTreeMap<String, String>, sort: i16) -> anyhow::Result<AiMcpServer> {
        let required: Vec<McpMarketEnvKey> = match item.env_keys.as_deref().map(str::trim).filter(|text| !text.is_empty()) {
            Some(text) => serde_json::from_str(text).context("mcp market envKeys is not a JSON array")?,
            None => Vec::new(),
        };
        for key in required.iter().filter(|key| key.required) {
            if env.get(&key.key).map(|value| value.trim().is_empty()).unwrap_or(true) {
                return Err(anyhow::anyhow!("env {} is required", key.key));
            }
        }
        let extra_args: Vec<String> = match item.args.as_deref().map(str::trim).filter(|text| !text.is_empty()) {
            Some(text) => serde_json::from_str(text).context("mcp market args is not a JSON array")?,
            None => Vec::new(),
        };
        let pkg = item.pkg.as_deref().map(str::trim).filter(|text| !text.is_empty());
        let env_json = if env.is_empty() { None } else { Some(serde_json::to_string(&env)?) };
        let mut server = AiMcpServer::default();
        server.sort = sort;
        server.name = item.name.clone();
        server.enabled = true;
        server.transport = MCP_TRANSPORT_STDIO;
        server.env = env_json;
        match item.runtime {
            MCP_MARKET_RUNTIME_NODE => {
                let pkg = pkg.ok_or_else(|| anyhow::anyhow!("mcp market item {} has no npm package", item.name))?;
                let mut args = vec!["-y".to_string(), pkg.to_string()];
                args.extend(extra_args);
                server.command = Some("npx".to_string());
                server.args = Some(serde_json::to_string(&args)?);
            }
            MCP_MARKET_RUNTIME_PYTHON => {
                let pkg = pkg.ok_or_else(|| anyhow::anyhow!("mcp market item {} has no python package", item.name))?;
                let mut args = vec![pkg.to_string()];
                args.extend(extra_args);
                server.command = Some("uvx".to_string());
                server.args = Some(serde_json::to_string(&args)?);
            }
            MCP_MARKET_RUNTIME_BINARY => {
                let command = pkg.ok_or_else(|| anyhow::anyhow!("mcp market item {} has no command", item.name))?;
                server.command = Some(command.to_string());
                server.args = if extra_args.is_empty() { None } else { Some(serde_json::to_string(&extra_args)?) };
            }
            MCP_MARKET_RUNTIME_HTTP => {
                let url = item.url.as_deref().map(str::trim).filter(|text| !text.is_empty()).ok_or_else(|| anyhow::anyhow!("mcp market item {} has no url", item.name))?;
                server.transport = MCP_TRANSPORT_HTTP;
                server.url = Some(url.to_string());
                server.env = None;
                if !env.is_empty() {
                    server.headers = Some(serde_json::to_string(&env)?);
                }
            }
            other => return Err(anyhow::anyhow!("unsupported mcp market runtime: {}", other)),
        }
        Ok(server)
    }

    fn normalize_server(&self, server: &mut AiMcpServer) -> anyhow::Result<()> {
        let name = server.name.trim().to_string();
        if name.is_empty() {
            return Err(ResultCode::NameEmpty.into());
        }
        server.name = name;
        server.command = normalize_optional_text(server.command.take());
        server.args = normalize_optional_text(server.args.take());
        server.env = normalize_optional_text(server.env.take());
        server.url = normalize_optional_text(server.url.take());
        server.headers = normalize_optional_text(server.headers.take());

        match server.transport {
            MCP_TRANSPORT_STDIO => {
                if server.command.is_none() {
                    return Err(ResultCode::McpStdioCommandRequired.into());
                }
            }
            MCP_TRANSPORT_HTTP => {
                if server.url.is_none() {
                    return Err(ResultCode::McpHttpUrlRequired.into());
                }
            }
            _ => {
                return Err(ResultCode::McpTransportUnsupported.into());
            }
        }

        if let Some(args) = server.args.as_deref() {
            validate_json_array(args, "args")?;
        }
        if let Some(env) = server.env.as_deref() {
            validate_json_object(env, "env")?;
        }
        if let Some(headers) = server.headers.as_deref() {
            validate_json_object(headers, "headers")?;
        }
        if let Some(timeout) = server.timeout_secs {
            if timeout <= 0 {
                return Err(ResultCode::TimeoutSecsInvalid.into());
            }
        }
        Ok(())
    }
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    let Some(text) = value else {
        return None;
    };
    let trimmed = text.trim().to_string();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed)
}

fn validate_json_array(raw: &str, _field: &str) -> anyhow::Result<()> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|_| anyhow::Error::from(ResultCode::JsonArrayRequired))?;
    if !value.is_array() {
        return Err(ResultCode::JsonArrayRequired.into());
    }
    for item in value.as_array().into_iter().flatten() {
        if !item.is_string() {
            return Err(ResultCode::JsonArrayRequired.into());
        }
    }
    Ok(())
}

fn validate_json_object(raw: &str, _field: &str) -> anyhow::Result<()> {
    let value: serde_json::Value = serde_json::from_str(raw).map_err(|_| anyhow::Error::from(ResultCode::JsonObjectRequired))?;
    let Some(object) = value.as_object() else {
        return Err(ResultCode::JsonObjectRequired.into());
    };
    for item in object.values() {
        if !item.is_string() {
            return Err(ResultCode::JsonObjectRequired.into());
        }
    }
    Ok(())
}
