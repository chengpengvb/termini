use std::path::Path;
use summer::tracing::info;
use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use termini_macros::base_app;
use std::collections::BTreeMap;
use crate::base::base_app::BaseApp;
use crate::model::ai_mcp_server::AiMcpServer;
use crate::module::ai::mcp::dto::mcp_transport_kind::MCP_TRANSPORT_STDIO;
use crate::module::ai::tools::common::document::external_tool::ExternalTool;
use crate::module::ai::tools::common::document::tool_installer::ToolInstaller;

const UV_TOOL_DIR: &str = "tools/uv-tools";
const UV_CACHE_DIR: &str = "tools/uv-cache";
const UV_PYTHON_DIR: &str = "tools/uv-python";
const DENO_CACHE_DIR: &str = "tools/deno-cache";

#[base_app(sort = 765)]
#[derive(Clone, Service)]
pub struct McpCommandResolver {
    #[inject(component)]
    tool_installer: ToolInstaller,
}

#[async_trait]
impl BaseApp for McpCommandResolver {
    fn name(&self) -> &'static str {
        "McpCommandResolver"
    }
}

impl McpCommandResolver {
    pub async fn resolve(&self, server: &AiMcpServer) -> Result<AiMcpServer> {
        if server.transport != MCP_TRANSPORT_STDIO {
            return Ok(server.clone());
        }
        let command = server.command.as_deref().map(str::trim).unwrap_or_default();
        match Self::launcher_name(command) {
            "npx" if Self::find_on_path(&["npx.cmd", "npx"]).is_none() => self.rewrite_to_deno(server).await,
            "uvx" if Self::find_on_path(&["uvx.exe", "uvx"]).is_none() => self.rewrite_to_uv(server).await,
            _ => Ok(server.clone()),
        }
    }

    async fn rewrite_to_deno(&self, server: &AiMcpServer) -> Result<AiMcpServer> {
        let deno = self.tool_installer.ensure(ExternalTool::Deno).await.map_err(|error| anyhow!(error.to_string()))?;
        let args = parse_args(server.args.as_deref())?;
        let mut rest = args.into_iter().filter(|arg| arg != "-y" && arg != "--yes").collect::<Vec<String>>();
        if rest.is_empty() {
            return Err(anyhow!("npx launch requires a package name in args"));
        }
        let package = rest.remove(0);
        let mut next_args = vec!["run".to_string(), "--allow-all".to_string(), "--quiet".to_string(), "--no-lock".to_string(), "--node-modules-dir=none".to_string(), format!("npm:{}", package)];
        next_args.extend(rest);
        let data_dir = ToolInstaller::data_dir().map_err(|error| anyhow!(error.to_string()))?;
        let mut env = parse_env(server.env.as_deref())?;
        env.insert("DENO_DIR".to_string(), data_dir.join(DENO_CACHE_DIR).display().to_string());
        env.insert("DENO_NO_UPDATE_CHECK".to_string(), "1".to_string());
        info!("[mcp] server={} 本机无 npx, 改用托管 deno 运行 npm:{}", server.name, package);
        Ok(Self::with_launch(server, &deno, next_args, env)?)
    }

    async fn rewrite_to_uv(&self, server: &AiMcpServer) -> Result<AiMcpServer> {
        let uv = self.tool_installer.ensure(ExternalTool::Uv).await.map_err(|error| anyhow!(error.to_string()))?;
        let args = parse_args(server.args.as_deref())?;
        if args.is_empty() {
            return Err(anyhow!("uvx launch requires a package name in args"));
        }
        let mut next_args = vec!["tool".to_string(), "run".to_string()];
        next_args.extend(args);
        let data_dir = ToolInstaller::data_dir().map_err(|error| anyhow!(error.to_string()))?;
        let mut env = parse_env(server.env.as_deref())?;
        env.insert("UV_PYTHON_INSTALL_DIR".to_string(), data_dir.join(UV_PYTHON_DIR).display().to_string());
        env.insert("UV_CACHE_DIR".to_string(), data_dir.join(UV_CACHE_DIR).display().to_string());
        env.insert("UV_TOOL_DIR".to_string(), data_dir.join(UV_TOOL_DIR).display().to_string());
        env.insert("UV_PYTHON_PREFERENCE".to_string(), "managed".to_string());
        info!("[mcp] server={} 本机无 uvx, 改用托管 uv 运行", server.name);
        Ok(Self::with_launch(server, &uv, next_args, env)?)
    }

    fn with_launch(server: &AiMcpServer, program: &Path, args: Vec<String>, env: BTreeMap<String, String>) -> Result<AiMcpServer> {
        let mut next = server.clone();
        next.command = Some(program.display().to_string());
        next.args = Some(serde_json::to_string(&args)?);
        next.env = Some(serde_json::to_string(&env)?);
        Ok(next)
    }

    fn launcher_name(command: &str) -> &str {
        let file = command.rsplit(['/', '\\']).next().unwrap_or(command);
        file.strip_suffix(".cmd").or_else(|| file.strip_suffix(".exe")).unwrap_or(file)
    }

    fn find_on_path(names: &[&str]) -> Option<std::path::PathBuf> {
        let path = std::env::var_os("PATH")?;
        for dir in std::env::split_paths(&path) {
            for name in names {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        None
    }
}

fn parse_args(raw: Option<&str>) -> Result<Vec<String>> {
    let Some(text) = raw.map(str::trim).filter(|text| !text.is_empty()) else {
        return Ok(Vec::new());
    };
    serde_json::from_str::<Vec<String>>(text).map_err(|error| anyhow!("args must be a JSON array of strings: {}", error))
}

fn parse_env(raw: Option<&str>) -> Result<BTreeMap<String, String>> {
    let Some(text) = raw.map(str::trim).filter(|text| !text.is_empty()) else {
        return Ok(BTreeMap::new());
    };
    serde_json::from_str::<BTreeMap<String, String>>(text).map_err(|error| anyhow!("env must be a JSON object of strings: {}", error))
}
