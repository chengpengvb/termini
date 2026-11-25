use summer::plugin::Service;
use async_trait::async_trait;
use crate::module::ai::shared::ToolBridge;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::req::req_tool_bridge::ReqToolBridgeCall;

const BACKEND_PORT: &str = env!("BACKEND_PORT");
const BRIDGE_PATH: &str = "/tool_bridge/call";

#[base_app(sort = 421)]
#[derive(Clone, Service)]
pub struct ToolBridgeController {
    #[inject(component)]
    tool_bridge: ToolBridge,
}

#[async_trait]
impl BaseApp for ToolBridgeController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "ToolBridgeController"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.tool_bridge.set_endpoint(format!("http://127.0.0.1:{}{}", BACKEND_PORT, BRIDGE_PATH));
        Ok(())
    }
}

#[control_routes(prefix = "/tool_bridge")]
impl ToolBridgeController {
    #[summer_web::post("/call")]
    async fn call(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqToolBridgeCall>) -> ResultModel<String> {
        ResultModel::from_result(self.tool_bridge.call(&body.token, &body.name, body.args).await)
    }
}
