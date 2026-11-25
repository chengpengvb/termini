use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use crate::model::req::req_system_util;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::service::service_diagnostics::DiagnosticsService;
use crate::model::resp::resp_platform_info::RespPlatformInfo;
use crate::model::resp::resp_diagnostics::RespDiagnosticsExport;
use crate::service::service_impl_system_util::SystemUtilServiceImpl;

#[base_app(sort = 420)]
#[derive(Clone, Service)]
pub struct SystemUtilController {
    #[inject(component)]
    diagnostics_service: DiagnosticsService,

    #[inject(component)]
    system_util_service_impl: SystemUtilServiceImpl,
}

#[async_trait]
impl BaseApp for SystemUtilController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "SystemUtilController"
    }
}

#[control_routes(prefix = "/system")]
impl SystemUtilController {
    #[summer_web::get("/platform/info")]
    async fn open(&self) -> ResultModel<RespPlatformInfo> {
        ResultModel::from_result(self.system_util_service_impl.platform_info().await)
    }

    #[summer_web::get("/master_key/phrase")]
    async fn master_key_phrase(&self) -> ResultModel<String> {
        ResultModel::from_result(self.system_util_service_impl.master_key_phrase().await)
    }

    #[summer_web::get("/auto_start")]
    async fn auto_start(&self) -> ResultModel<bool> {
        ResultModel::from_result(self.system_util_service_impl.auto_start().await)
    }

    #[summer_web::post("/auto_start")]
    async fn save_auto_start(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqAutoStart>) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.save_auto_start(body.enabled).await)
    }

    #[summer_web::post("/open_here")]
    async fn save_open_here(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqOpenHere>) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.save_open_here(body.enabled).await)
    }

    #[summer_web::get("/clipboard/local-paths")]
    async fn read_clipboard_local_paths(&self) -> ResultModel<Vec<String>> {
        ResultModel::from_result(self.system_util_service_impl.read_clipboard_local_paths().await)
    }

    #[summer_web::post("/clipboard/text")]
    async fn write_clipboard_text(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqClipboardText>) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.write_clipboard_text(&body.text).await)
    }

    #[summer_web::get("/clipboard/text")]
    async fn read_clipboard_text(&self) -> ResultModel<String> {
        ResultModel::from_result(self.system_util_service_impl.read_clipboard_text().await)
    }

    #[summer_web::post("/open_url")]
    async fn open_url(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqOpenUrl>) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.open_url(&body.url).await)
    }

    #[summer_web::post("/open_path")]
    async fn open_path(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqLocalPath>) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.open_local_path(&body.path).await)
    }

    #[summer_web::post("/reveal_path")]
    async fn reveal_path(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqLocalPath>) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.reveal_local_path(&body.path).await)
    }

    #[summer_web::post("/copy_local_into")]
    async fn copy_local_into(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqLocalCopyInto>) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.copy_local_into(&body.dest_dir, &body.sources).await)
    }

    #[summer_web::post("/haptic")]
    async fn haptic(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqHaptic>) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.haptic(&body).await)
    }

    #[summer_web::post("/notify")]
    async fn notify(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqNotify>) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.notify(&body.title, &body.body).await)
    }

    #[summer_web::post("/diagnostics/export")]
    async fn export_diagnostics(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqDiagnosticsExport>) -> ResultModel<RespDiagnosticsExport> {
        ResultModel::from_result(self.diagnostics_service.export(&body.path).await)
    }

    #[summer_web::post("/window/minimize")]
    async fn window_minimize(&self) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.window_minimize().await)
    }

    #[summer_web::post("/window/request_attention")]
    async fn window_request_attention(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqWindowAttention>) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.window_request_attention(body.active).await)
    }

    #[summer_web::post("/window/toggle_maximize")]
    async fn window_toggle_maximize(&self) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.window_toggle_maximize().await)
    }

    #[summer_web::post("/window/close")]
    async fn window_close(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqWindowClose>) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.window_close(body.action).await)
    }

    #[summer_web::get("/window/maximized")]
    async fn window_maximized(&self) -> ResultModel<bool> {
        ResultModel::from_result(self.system_util_service_impl.window_is_maximized().await)
    }

    #[summer_web::get("/window/fullscreen")]
    async fn window_fullscreen(&self) -> ResultModel<bool> {
        ResultModel::from_result(self.system_util_service_impl.window_is_fullscreen().await)
    }

    #[summer_web::post("/window/fullscreen")]
    async fn window_set_fullscreen(&self, summer_web::axum::Json(body): summer_web::axum::Json<req_system_util::ReqWindowFullscreen>) -> ResultModel<()> {
        ResultModel::from_result(self.system_util_service_impl.window_set_fullscreen(body.fullscreen).await)
    }
}
