use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use crate::module::serial::available_ports;
use crate::model::config_serial::ConfigSerial;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::model::req::req_serial::ReqSerialConfig;
use crate::service::service_config_serial::ConfigSerialService;

#[base_app(sort = 421)]
#[derive(Clone, Service)]
pub struct SerialController {
    #[inject(component)]
    config_serial_service: ConfigSerialService,
}

#[async_trait]
impl BaseApp for SerialController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "SerialController"
    }
}

#[control_routes(prefix = "/serial")]
impl SerialController {
    #[summer_web::get("/ports")]
    async fn ports(&self) -> ResultModel<Vec<String>> {
        ResultModel::from_result(Ok(available_ports()))
    }

    #[summer_web::post("/config/get")]
    async fn config_get(&self, summer_web::axum::Json(body): summer_web::axum::Json<ReqSerialConfig>) -> ResultModel<Option<ConfigSerial>> {
        ResultModel::ok(self.config_serial_service.get_by_port_name(body.port_name.as_str()).map(|ptr| ptr.read().clone()))
    }

    #[summer_web::post("/config/save")]
    async fn config_save(&self, summer_web::axum::Json(body): summer_web::axum::Json<ConfigSerial>) -> ResultModel<ConfigSerial> {
        ResultModel::from_result(self.config_serial_service.save_by_port(body).await.map(|ptr| ptr.read().clone()))
    }
}
