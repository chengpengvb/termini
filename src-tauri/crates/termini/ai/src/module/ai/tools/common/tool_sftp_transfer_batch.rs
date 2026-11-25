use std::sync::Arc;
use serde_json::json;
use std::path::PathBuf;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::util::id_factory::uuid;
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use crate::module::ai::dto::AiToolId;
use crate::module::ai::dto::AiRunContext;
use crate::module::ai::shared::ToolCallDonePush;
use rig::tool::{DynamicTool, Tool, ToolContext};
use crate::module::ai::shared::ToolCallChunkPush;
use crate::module::ai::tools::tool_base::ToolBase;
use crate::service::service_device::DeviceService;
use crate::module::ai::shared::ToolConflictManager;
use crate::model::dto::cancel_session::CancelSession;
use crate::module::ai::shared::AgentRemoteClientPool;
use crate::module::ai::shared::ChatSessionCacheManager;
use crate::module::ai::tools::tool_manager::ToolManager;
use crate::web::ws::handle::r#impl::sftp::SessionClient;
use crate::module::ai::tools::ai_tool_error::AiToolError;
use crate::web::ws::handle::r#impl::handle_sftp::HandleSftp;
use crate::web::ws::handle::r#impl::sftp::RemoteSessionClient;
use crate::module::ai::tools::base::{ToolBaseOutWrap, WrapItem};
use crate::module::ai::tools::common::remote::device_batch::{self, MAX_DEVICE_IDS};
use crate::web::ws::handle::r#impl::sftp::handle_transfer::{TransferCoreHooks, TransferCoreInput, TransferEventCallback};
use crate::module::ai::tools::common::remote::sftp_paths::{self, SFTP_TRANSFER_DIRECTION_DOWNLOAD, SFTP_TRANSFER_DIRECTION_UPLOAD};

const BATCH_CONCURRENCY: usize = 4;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct ToolSftpTransferBatchArgs {
    pub local_path: String,
    pub remote_path: String,
    pub device_ids: Vec<String>,

    #[serde(default)]
    pub force_overwrite: bool,

    #[serde(default)]
    pub direction: i16,

    #[serde(default, rename = "__internal_call_id")]
    pub internal_call_id: Option<String>,
}

#[base_app(sort = 767)]
#[derive(Clone, Service)]
pub struct ToolSftpTransferBatch {
    #[inject(component)]
    handle_sftp: HandleSftp,

    #[inject(component)]
    tool_manager: ToolManager,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    tool_call_done_push: ToolCallDonePush,

    #[inject(component)]
    tool_call_chunk_push: ToolCallChunkPush,

    #[inject(component)]
    tool_conflict_manager: ToolConflictManager,

    #[inject(component)]
    agent_remote_client_pool: AgentRemoteClientPool,

    #[inject(component)]
    chat_session_cache_manager: ChatSessionCacheManager,
}

#[async_trait]
impl BaseApp for ToolSftpTransferBatch {
    fn name(&self) -> &'static str {
        "ToolSftpTransferBatch"
    }

    async fn start(&self, _ctx: &crate::base::base_app::AppContext) -> anyhow::Result<()> {
        self.tool_manager.register(Arc::new(self.clone()));
        Ok(())
    }
}

impl ToolBase for ToolSftpTransferBatch {
    fn tool_id(&self) -> AiToolId {
        AiToolId::ToolSftpTransferBatch
    }

    fn on_stop(&self, req_id: &str) {
        self.agent_remote_client_pool.release(req_id);
    }

    fn persist(&self) -> bool {
        true
    }

    fn as_rig_tool(&self, done_push: Option<ToolCallDonePush>) -> DynamicTool {
        ToolBaseOutWrap::wrap(self.clone(), self.max_result_tokens(), done_push)
    }
}

impl ToolSftpTransferBatch {
    fn device_download_dir(local_root: &str, name: &str, device_id: &str) -> Result<String, String> {
        let safe: String = name.chars().map(|ch| if ch.is_alphanumeric() || ch == '-' || ch == '_' || ch == '.' || (ch as u32) > 127 { ch } else { '_' }).collect();
        let folder = if safe.trim_matches('_').is_empty() { device_id.to_string() } else { safe };
        let dir = PathBuf::from(local_root).join(folder);
        std::fs::create_dir_all(&dir).map_err(|error| format!("create {}: {}", dir.display(), error))?;
        Ok(dir.to_string_lossy().to_string())
    }

    fn progress_hooks(&self, run_context: Arc<AiRunContext>, call_id: String, device_id: String) -> TransferCoreHooks {
        let pusher = self.tool_call_chunk_push.clone();
        let tool_id = self.tool_id().as_str().to_string();
        let on_event: TransferEventCallback = Arc::new(move |event| {
            let pusher = pusher.clone();
            let run_context = Arc::clone(&run_context);
            let call_id = call_id.clone();
            let tool_id = tool_id.clone();
            let device_id = device_id.clone();
            Box::pin(async move {
                let Some(delta) = sftp_paths::format_progress_event(&event) else {
                    return;
                };
                let _ = pusher.push_for_device(&run_context, &call_id, &tool_id, Some(device_id), delta);
            })
        });
        TransferCoreHooks { on_event, on_conflict: None }
    }
}

impl Tool for ToolSftpTransferBatch {
    const NAME: &'static str = AiToolId::ToolSftpTransferBatch.as_str();

    type Error = AiToolError;
    type Args = ToolSftpTransferBatchArgs;
    type Output = String;

    fn description(&self) -> String {
        format!("经 SFTP 在宿主机与一台或多台已登记设备之间传输文件/目录。direction=0（默认）上传：把本地文件/目录下发到每台设备的 remote_path 目录；direction=1 下载：从每台设备取同一 remote_path 回本地 local_path 目录，多台时每台放进以设备名命名的子目录。整批一次确认，按机汇总成败，单机失败不影响其余设备。目标必须是已登记设备 device_id（由引用注入中的 group/设备展开，最多 {} 台，不接受 group_id），不能是本机。force_overwrite 默认 false：目标侧已存在同名文件的那台会报错，需再次调用并设为 true 才覆盖。", MAX_DEVICE_IDS)
    }

    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "device_ids": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "已登记设备 id 列表（一台也传数组）；由引用目标展开，不要传 group_id；会去重保序"
                },
                "local_path": {
                    "type": "string",
                    "description": "宿主机本地绝对路径。上传：源文件/目录；下载：已存在的目标目录（每台设备一个子目录）"
                },
                "remote_path": {
                    "type": "string",
                    "description": "各设备上的远端绝对路径。上传：目标目录；下载：源文件/目录"
                },
                "direction": {
                    "type": "integer",
                    "description": "0=上传(本地→各设备，默认)，1=下载(各设备→本地)"
                },
                "force_overwrite": {
                    "type": "boolean",
                    "description": "是否强制覆盖目标侧已存在文件；默认 false，冲突的那台会报错"
                }
            },
            "required": ["device_ids", "local_path", "remote_path"]
        })
    }

    async fn call(&self, context: &mut ToolContext, args: Self::Args) -> anyhow::Result<Self::Output, Self::Error> {
        let run_context = context.require::<Arc<AiRunContext>>().map_err(|error| AiToolError::Execution(error.to_string()))?.clone();
        let Some(call_id) = args.internal_call_id.clone() else {
            return Err(AiToolError::Execution("internal_call_id of the tool call is missing".to_string()));
        };
        let device_ids = device_batch::normalize_device_ids(args.device_ids)?;
        let direction = sftp_paths::resolve_direction(args.direction)?;
        if direction == SFTP_TRANSFER_DIRECTION_UPLOAD {
            sftp_paths::resolve_local_upload(&args.local_path)?;
            if args.remote_path.trim().is_empty() {
                return Err(AiToolError::Execution("remote_path is empty".to_string()));
            }
        } else {
            sftp_paths::resolve_local_download_target(&args.local_path)?;
            sftp_paths::resolve_remote_source(&args.remote_path)?;
        }
        let session_cancel = run_context.cancel_token.clone();
        let Some(tool_cancel) = self.chat_session_cache_manager.get_tool_cancel(&run_context.req_id, &run_context.run_id, &call_id) else {
            return Err(AiToolError::Execution(format!("tool cancel not found: req_id={} run_id={} call_id={}", run_context.req_id, run_context.run_id, call_id)));
        };
        let confirm_args = ToolSftpTransferBatchArgs {
            device_ids: device_ids.clone(),
            local_path: args.local_path.clone(),
            remote_path: args.remote_path.clone(),
            force_overwrite: args.force_overwrite,
            direction,
            internal_call_id: None,
        };
        self.tool_conflict_manager.confirm_before_execute(&run_context, &call_id, self.tool_id().as_str(), &confirm_args).await?;

        let multi = device_ids.len() > 1;
        let tool = self.clone();
        let run = Arc::clone(&run_context);
        let call_id_owned = call_id.clone();
        let local_path = args.local_path.clone();
        let remote_path = args.remote_path.clone();
        let force_overwrite = args.force_overwrite;
        let tool_cancel_token = tool_cancel.clone();
        let session_cancel_token = session_cancel.clone();
        let items = device_batch::run_device_batch(device_ids, BATCH_CONCURRENCY, &tool_cancel, &session_cancel, move |device_id: String| {
            let tool = tool.clone();
            let run = Arc::clone(&run);
            let call_id = call_id_owned.clone();
            let local_path = local_path.clone();
            let remote_path = remote_path.clone();
            let tool_cancel_token = tool_cancel_token.clone();
            let session_cancel_token = session_cancel_token.clone();
            async move {
                let (name, host) = match device_batch::device_label(&tool.device_service, &device_id).await {
                    Ok(label) => label,
                    Err(error) => return device_batch::error_item(device_id, String::new(), String::new(), error),
                };
                let remote = match tool.agent_remote_client_pool.get_or_open_sftp(&run.req_id, &device_id).await {
                    Ok(remote) => remote,
                    Err(error) => return device_batch::error_item(device_id, name, host, error.detail().to_string()),
                };
                let local = tool.handle_sftp.local_session_client();
                let plan = if direction == SFTP_TRANSFER_DIRECTION_DOWNLOAD {
                    sftp_paths::resolve_remote_source(&remote_path).map_err(|error| error.to_string()).and_then(|(source_path, file_name)| {
                        let target = if multi { Self::device_download_dir(&local_path, &name, &device_id)? } else { local_path.clone() };
                        Ok((SessionClient::Remote(RemoteSessionClient::Sftp(remote)), local, source_path, file_name, target.clone(), format!("downloaded {}:{} -> {}", name, remote_path, target)))
                    })
                } else {
                    sftp_paths::resolve_local_upload(&local_path).map_err(|error| error.to_string()).map(|(source_path, file_name)| {
                        (local, SessionClient::Remote(RemoteSessionClient::Sftp(remote)), source_path, file_name, remote_path.clone(), format!("uploaded {} -> {}:{}", local_path, name, remote_path))
                    })
                };
                let (source, target, source_path, file_name, target_path, summary) = match plan {
                    Ok(plan) => plan,
                    Err(error) => return device_batch::error_item(device_id, name, host, error),
                };
                let session = Arc::new(CancelSession::new());
                let hooks = tool.progress_hooks(Arc::clone(&run), call_id, device_id.clone());
                let input = TransferCoreInput {
                    source_path,
                    resp_id: uuid(),
                    names: vec![file_name],
                    target_path,
                    force_overwrite: Some(force_overwrite),
                    client_transfer_id: tool.handle_sftp.local_client_transfer_id(),
                };
                let transfer = tool.handle_sftp.execute_transfer_core(&source, &target, input, Arc::clone(&session), &hooks);
                tokio::pin!(transfer);
                tokio::select! {
                    biased;
                    res = &mut transfer => match res {
                        Ok(_) => device_batch::ok_item(device_id, name, host, None, summary),
                        Err(error) => device_batch::error_item(device_id, name, host, error.to_string()),
                    },
                    _ = tool_cancel_token.cancelled() => {
                        session.cancel();
                        device_batch::cancelled_item(device_id, name, host)
                    }
                    _ = session_cancel_token.cancelled() => {
                        session.cancel();
                        device_batch::cancelled_item(device_id, name, host)
                    }
                }
            }
        })
        .await;

        let json = serde_json::to_string(&items).map_err(|error| AiToolError::Execution(error.to_string()))?;
        self.tool_call_done_push.push_tool_call_done_event(&run_context, &call_id, json);
        let mut wrap_items: Vec<WrapItem> = Vec::with_capacity(items.len());
        for item in &items {
            let body = serde_json::to_string(item).map_err(|error| AiToolError::Execution(error.to_string()))?;
            wrap_items.push(WrapItem::wrap_item(Some(format!("===== {} =====", item.device_id)), body));
        }
        context.insert_result(wrap_items);
        Ok(String::new())
    }
}
