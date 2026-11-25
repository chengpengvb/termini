use std::sync::Arc;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::web::ws::ws_manager::WsManager;
use crate::base::base_app::{AppContext, BaseApp};
use summer_web::socketioxide::extract::SocketRef;
use crate::model::req::req_ws_context::ReqSocketContext;
use crate::model::resp::resp_ws_context::RespSocketContext;
use crate::module::common::context::context_shell::ShellDescr;
use crate::model::resp::event::resp_event_shell::RespEventShell;
use crate::module::shell::platform::shell_discover::{native_shell_platform, ShellDiscover};

fn build_native_shell_platform() -> Arc<dyn ShellDiscover + Send + Sync> {
    native_shell_platform()
}

#[derive(Clone, Service)]
pub struct ShellService {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(func = build_native_shell_platform())]
    shell_discover: Arc<dyn ShellDiscover + Send + Sync>,
}

#[async_trait]
impl BaseApp for ShellService {
    fn name(&self) -> &'static str {
        "ShellService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }
}

impl ShellService {
    pub fn discover(&self) -> Vec<ShellDescr> {
        self.shell_discover.discover_cached()
    }

    pub fn find_by_shell_id(&self, shell_id: &str) -> Option<ShellDescr> {
        self.shell_discover.find_by_shell_id(shell_id)
    }

    pub fn handler_shell_event(&self, socket: SocketRef, req: Arc<ReqSocketContext>, event: RespEventShell) {
        let mut resp = RespSocketContext { r#type: event.to_type(), session_id: req.session_id.clone(), payload: None };
        match event {
            RespEventShell::OnOpen => self.ws_manager.send_msg(socket, &resp),
            RespEventShell::OnClose => self.ws_manager.send_msg(socket, &resp),
            RespEventShell::OnError(err) => {
                resp.payload = Some(serde_json::to_value(err.to_string()).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
            RespEventShell::OnData(data) => {
                resp.payload = Some(serde_json::to_value(data.clone()).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
            RespEventShell::Exit { code } => {
                resp.payload = Some(serde_json::to_value(code).unwrap());
                self.ws_manager.send_msg(socket, &resp);
            }
            RespEventShell::SnapshotRequest => self.ws_manager.send_msg(socket, &resp),
        }
    }
}
