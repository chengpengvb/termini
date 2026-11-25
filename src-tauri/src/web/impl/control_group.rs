use summer_web::axum;
use summer::plugin::Service;
use crate::model::group::Group;
use crate::model::dto::shared_ptr::Shared;
use crate::web::control_base::BaseControl;
use crate::model::req::req_group::ReqGroupSet;
use crate::model::resp::resp_model::ResultModel;
use crate::service::service_group::GroupService;
use termini_macros::{base_control, control_routes};
use crate::service::service_group_2_device::Group2DeviceService;

#[base_control(Group, prefix = "/group", skip = "insert,update,delete")]
#[derive(Clone, Service)]
pub struct GroupController {
    #[inject(component)]
    svc: GroupService,

    #[inject(component)]
    group_2_device_service: Group2DeviceService,
}

impl BaseControl for GroupController {
    type Service = GroupService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}

#[control_routes(prefix = "/group")]
impl GroupController {
    #[summer_web::post("/insert")]
    async fn insert(&self, axum::Json(body): axum::Json<Group>) -> ResultModel<Shared<Group>> {
        ResultModel::from_result(self.svc.create_group(body).await)
    }

    #[summer_web::post("/update")]
    async fn update(&self, axum::Json(body): axum::Json<Group>) -> ResultModel<Shared<Group>> {
        ResultModel::from_result(self.svc.update_group(body).await)
    }

    #[summer_web::post("/set_device_2_group")]
    async fn set_device_2_group(&self, axum::Json(body): axum::Json<ReqGroupSet>) -> ResultModel<()> {
        ResultModel::from_result(self.group_2_device_service.set_device_2_group(&body.group_id, &body.device_id).await)
    }

    #[summer_web::delete("/delete/{id}")]
    async fn delete(&self, axum::extract::Path(id): axum::extract::Path<String>) -> ResultModel<()> {
        ResultModel::from_result(self.svc.delete_group(id).await)
    }
}
