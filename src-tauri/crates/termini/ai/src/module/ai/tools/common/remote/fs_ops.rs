use std::sync::Arc;
use serde_json::json;
use crate::util::path_util;
use serde::{Deserialize, Serialize};
use crate::module::common::dto::file_entry::FileKind;
use crate::web::ws::handle::r#impl::sftp::SessionClient;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::module::ai::tools::common::remote::sftp_paths;
use crate::web::ws::handle::r#impl::handle_sftp::HandleSftp;
use crate::model::resp::resp_sftp::RespSftpDeleteProgressHandler;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolFsArgs {
    pub op: String,
    pub device_id: String,

    #[serde(default)]
    pub path: Option<String>,

    #[serde(default)]
    pub name: Option<String>,

    #[serde(default)]
    pub names: Option<Vec<String>>,

    #[serde(default)]
    pub from: Option<String>,

    #[serde(default)]
    pub to: Option<String>,

    #[serde(default)]
    pub kind: Option<String>,

    #[serde(default)]
    pub mode: Option<u32>,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

pub fn is_write(op: &str) -> bool {
    matches!(op, "rename" | "delete" | "create" | "chmod")
}

fn require_abs(label: &str, value: Option<&String>) -> Result<String, AiToolError> {
    let Some(raw) = value else {
        return Err(AiToolError::Execution(format!("{} is required", label)));
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AiToolError::Execution(format!("{} is empty", label)));
    }
    if !trimmed.starts_with('/') {
        return Err(AiToolError::Execution(format!("{} must be absolute: {}", label, raw)));
    }
    Ok(trimmed.to_string())
}

fn require_names(names: Option<&Vec<String>>) -> Result<Vec<String>, AiToolError> {
    let Some(names) = names else {
        return Err(AiToolError::Execution("names is required".to_string()));
    };
    if names.is_empty() {
        return Err(AiToolError::Execution("names is empty".to_string()));
    }
    let mut out = Vec::with_capacity(names.len());
    for name in names {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(AiToolError::Execution("name is empty".to_string()));
        }
        out.push(trimmed.to_string());
    }
    Ok(out)
}

fn parse_kind(kind: Option<&String>) -> Result<FileKind, AiToolError> {
    let Some(kind) = kind else {
        return Err(AiToolError::Execution("kind is required".to_string()));
    };
    match kind.trim() {
        "file" => Ok(FileKind::File),
        "directory" => Ok(FileKind::Directory),
        other => Err(AiToolError::Execution(format!("kind must be file or directory, got {}", other))),
    }
}

fn delete_spec(path: Option<&String>, names: Option<&Vec<String>>) -> Result<(String, Vec<String>), AiToolError> {
    match names {
        Some(_) => Ok((require_abs("path", path)?, require_names(names)?)),
        None => {
            let full = require_abs("path", path)?;
            let (parent, name) = sftp_paths::resolve_remote_source(&full)?;
            Ok((parent, vec![name]))
        }
    }
}

fn create_path(path: Option<&String>, name: Option<&String>) -> Result<String, AiToolError> {
    let path = require_abs("path", path)?;
    let Some(name) = name else {
        return Ok(path);
    };
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AiToolError::Execution("name is empty".to_string()));
    }
    Ok(path_util::join_frontend_path(&path, trimmed))
}

fn chmod_paths(path: Option<&String>, names: Option<&Vec<String>>) -> Result<Vec<String>, AiToolError> {
    match names {
        Some(_) => {
            let parent = require_abs("path", path)?;
            Ok(require_names(names)?.into_iter().map(|name| path_util::join_frontend_path(&parent, &name)).collect())
        }
        None => Ok(vec![require_abs("path", path)?]),
    }
}

pub fn validate(op: &str, args: &ToolFsArgs, allow_chmod: bool) -> Result<(), AiToolError> {
    if args.device_id.trim().is_empty() {
        return Err(AiToolError::Execution("device_id is empty".to_string()));
    }
    match op {
        "list" => {
            require_abs("path", args.path.as_ref())?;
            Ok(())
        }
        "rename" => {
            require_abs("from", args.from.as_ref())?;
            require_abs("to", args.to.as_ref())?;
            Ok(())
        }
        "delete" => {
            delete_spec(args.path.as_ref(), args.names.as_ref())?;
            Ok(())
        }
        "create" => {
            create_path(args.path.as_ref(), args.name.as_ref())?;
            parse_kind(args.kind.as_ref())?;
            Ok(())
        }
        "chmod" => {
            if !allow_chmod {
                return Err(AiToolError::Execution("ftp does not support chmod".to_string()));
            }
            if args.mode.is_none() {
                return Err(AiToolError::Execution("mode is required".to_string()));
            }
            chmod_paths(args.path.as_ref(), args.names.as_ref())?;
            Ok(())
        }
        other => Err(AiToolError::Execution(format!("unknown op: {}", other))),
    }
}

fn noop_delete_handler() -> RespSftpDeleteProgressHandler {
    Arc::new(|_progress| Box::pin(async { Ok(()) }))
}

pub async fn execute(client: &SessionClient, op: &str, args: &ToolFsArgs) -> Result<serde_json::Value, AiToolError> {
    match op {
        "list" => {
            let path = require_abs("path", args.path.as_ref())?;
            let entries = client.list_dir(&path).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
            serde_json::to_value(json!({ "op": "list", "path": path, "entries": entries })).map_err(|error| AiToolError::Execution(error.to_string()))
        }
        "rename" => {
            let from = require_abs("from", args.from.as_ref())?;
            let to = require_abs("to", args.to.as_ref())?;
            client.rename(&from, &to).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
            Ok(json!({ "op": "rename", "from": from, "to": to }))
        }
        "delete" => {
            let (parent, names) = delete_spec(args.path.as_ref(), args.names.as_ref())?;
            let roots = HandleSftp::resolve_delete_roots(client, &parent, &names, None).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
            let plan = HandleSftp::collect_delete_plan(client, roots, None).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
            let count = plan.len();
            client.delete(&plan, None, noop_delete_handler()).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
            Ok(json!({ "op": "delete", "path": parent, "names": names, "count": count }))
        }
        "create" => {
            let path = create_path(args.path.as_ref(), args.name.as_ref())?;
            let kind = parse_kind(args.kind.as_ref())?;
            client.create(&path, kind, None).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
            Ok(json!({ "op": "create", "path": path, "kind": args.kind.as_ref().map(|value| value.trim()) }))
        }
        "chmod" => {
            let mode = args.mode.ok_or_else(|| AiToolError::Execution("mode is required".to_string()))?;
            let paths = chmod_paths(args.path.as_ref(), args.names.as_ref())?;
            client.chmod(&paths, mode & 0o7777).await.map_err(|error| AiToolError::Execution(error.to_string()))?;
            Ok(json!({ "op": "chmod", "mode": mode, "paths": paths }))
        }
        other => Err(AiToolError::Execution(format!("unknown op: {}", other))),
    }
}
