use summer_web::axum::Json;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::base::base_app::BaseApp;
use summer_web::axum::extract::Path;
use crate::service::service_sftp::SftpService;
use termini_macros::{base_app, control_routes};
use crate::model::resp::resp_model::ResultModel;
use crate::model::resp::resp_sftp::RespSftpLocalId;
use crate::model::resp::resp_sftp::RespSftpRemoteDrag;
use crate::service::service_sftp_drag::SftpDragService;
use crate::model::dto::ssh_identity_maps::SshIdentityMaps;
use crate::web::ws::handle::r#impl::handle_sftp::HandleSftp;
use crate::model::req::req_sftp::{ReqSftpStartDrag, ReqSftpStartRemoteDrag};
use crate::model::resp::resp_sftp_transfer::{RespSftpFileCopy, RespSftpFilePaste, SFTP_FILE_COPY_SOURCE_KIND_APP};

#[base_app(sort = 422)]
#[derive(Clone, Service)]
pub struct SftpController {
    #[inject(component)]
    sftp_handler: HandleSftp,

    #[inject(component)]
    sftp_service: SftpService,

    #[inject(component)]
    sftp_drag_service: SftpDragService,
}

#[async_trait]
impl BaseApp for SftpController {
    fn phase(&self) -> i16 {
        40
    }

    fn name(&self) -> &'static str {
        "SftpController"
    }
}

#[control_routes(prefix = "/sftp")]
impl SftpController {
    #[summer_web::get("/local_client_transfer_id")]
    async fn local_client_transfer_id(&self) -> ResultModel<RespSftpLocalId> {
        ResultModel::ok(RespSftpLocalId { client_transfer_id: self.sftp_handler.local_client_transfer_id() })
    }

    #[summer_web::post("/file_copy")]
    async fn set_file_copy(&self, Json(body): Json<RespSftpFileCopy>) -> ResultModel<()> {
        self.sftp_drag_service.set_file_copy(body, SFTP_FILE_COPY_SOURCE_KIND_APP);
        ResultModel::ok(())
    }

    #[summer_web::post("/file_paste")]
    async fn set_file_paste(&self, Json(body): Json<RespSftpFilePaste>) -> ResultModel<()> {
        self.sftp_drag_service.set_file_paste(body, SFTP_FILE_COPY_SOURCE_KIND_APP);
        ResultModel::ok(())
    }

    #[summer_web::post("/drag/start")]
    async fn start_drag(&self, Json(body): Json<ReqSftpStartDrag>) -> ResultModel<()> {
        ResultModel::from_result(self.sftp_drag_service.start_drag(body.paths))
    }

    #[summer_web::get("/identity/map/{device_id}")]
    async fn identity_map(&self, Path(device_id): Path<String>) -> ResultModel<SshIdentityMaps> {
        match self.sftp_service.get_identity_maps(&device_id) {
            Some(maps) => ResultModel::ok(maps),
            None => ResultModel::ok(SshIdentityMaps::default()),
        }
    }

    #[summer_web::post("/drag/start_remote")]
    async fn start_remote_drag(&self, Json(body): Json<ReqSftpStartRemoteDrag>) -> ResultModel<()> {
        let req = RespSftpRemoteDrag {
            names: body.names,
            #[cfg(target_os = "macos")]
            screen_x: body.screen_x,
            #[cfg(target_os = "macos")]
            screen_y: body.screen_y,
            session_id: body.session_id,
            source_path: body.source_path,
            client_transfer_id: body.client_transfer_id,
            local_client_transfer_id: self.sftp_handler.local_client_transfer_id(),
        };
        ResultModel::from_result(self.sftp_drag_service.start_remote_drag(req))
    }
}
