use summer_web::axum;
use summer::plugin::Service;
use crate::model::dto::shared_ptr::Shared;
use crate::web::control_base::BaseControl;
use crate::model::ai_mcp_server::AiMcpServer;
use crate::model::resp::resp_model::ResultModel;
use termini_macros::{base_control, control_routes};
use crate::model::server::mcp_market::McpMarketItem;
use crate::model::req::req_mcp_market::ReqMcpMarketInstall;
use crate::service::service_ai_mcp_server::AiMcpServerService;
use crate::module::ai::mcp::dto::mcp_tool_snapshot::McpToolSnapshot;
use crate::module::ai::mcp::dto::mcp_prompt_snapshot::McpPromptSnapshot;
use crate::model::resp::resp_ai_mcp_server::{RespAiMcpServerRuntime, RespAiMcpServerTools};

#[base_control(AiMcpServer, prefix = "/ai_mcp_server", skip = "insert,update,delete")]
#[derive(Clone, Service)]
pub struct AiMcpServerController {
    #[inject(component)]
    svc: AiMcpServerService,
}

impl BaseControl for AiMcpServerController {
    type Service = AiMcpServerService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/ai_mcp_server")]
impl AiMcpServerController {
    #[summer_web::post("/insert")]
    async fn insert(&self, axum::Json(body): axum::Json<AiMcpServer>) -> ResultModel<Shared<AiMcpServer>> {
        ResultModel::from_result(self.svc.create_server(body).await)
    }

    #[summer_web::post("/update")]
    async fn update(&self, axum::Json(body): axum::Json<AiMcpServer>) -> ResultModel<Shared<AiMcpServer>> {
        ResultModel::from_result(self.svc.update_server(body).await)
    }

    #[summer_web::delete("/delete/{id}")]
    async fn delete(&self, axum::extract::Path(id): axum::extract::Path<String>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.delete_server(id).await)
    }

    #[summer_web::get("/list_runtime")]
    async fn list_runtime(&self) -> ResultModel<Vec<RespAiMcpServerRuntime>> {
        ResultModel::from_result(self.svc.list_runtime().await)
    }

    #[summer_web::post("/probe")]
    async fn probe(&self, axum::Json(body): axum::Json<AiMcpServer>) -> ResultModel<Vec<McpToolSnapshot>> {
        ResultModel::from_result(self.svc.probe(body).await)
    }

    #[summer_web::post("/{id}/reconnect")]
    async fn reconnect(&self, axum::extract::Path(id): axum::extract::Path<String>) -> ResultModel<RespAiMcpServerRuntime> {
        ResultModel::from_result(self.svc.reconnect(id).await)
    }

    #[summer_web::get("/{id}/tools")]
    async fn tools(&self, axum::extract::Path(id): axum::extract::Path<String>) -> ResultModel<RespAiMcpServerTools> {
        ResultModel::from_result(self.svc.list_tools(id))
    }

    #[summer_web::get("/prompts")]
    async fn prompts(&self) -> ResultModel<Vec<McpPromptSnapshot>> {
        ResultModel::from_result(self.svc.list_prompts())
    }

    #[summer_web::get("/tools")]
    async fn connected_tools(&self) -> ResultModel<Vec<McpToolSnapshot>> {
        ResultModel::from_result(self.svc.list_connected_tools())
    }

    #[summer_web::get("/market/list")]
    async fn market_list(&self) -> ResultModel<Vec<McpMarketItem>> {
        ResultModel::from_result(self.svc.fetch_market().await)
    }

    #[summer_web::post("/market/install")]
    async fn market_install(&self, axum::Json(body): axum::Json<ReqMcpMarketInstall>) -> ResultModel<Shared<AiMcpServer>> {
        ResultModel::from_result(self.svc.install_from_market(body).await)
    }
}
