use std::sync::Arc;
use dashmap::DashMap;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::web::ws::ws_manager::WsManager;
use crate::model::port_forward::PortForward;
use summer::plugin::component::ComponentRef;
use crate::service::service_base::BaseService;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_device::DeviceService;
use crate::module::common::client_terminal::EventHandler;
use crate::service::service_config_ssh::ConfigSshService;
use crate::model::resp::resp_ws_context::RespSocketContext;
use crate::module::common::context::context_ssh::ContextSsh;
use crate::module::common::context::context_base::WindowSize;
use crate::service::service_port_forward::PortForwardService;
use crate::module::ssh_forward::client_forward::ClientForward;
use crate::model::resp::event::resp_event_forward::RespEventForward;
use crate::module::ssh::client_session_factory::ClientSessionFactory;
use crate::model::port_forward::{FORWARD_STATUS_ERROR, FORWARD_STATUS_IDLE, FORWARD_STATUS_RUNNING};

fn create_clients() -> Arc<DashMap<String, Arc<ClientForward>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 905)]
#[derive(Clone, Service)]
pub struct ForwardManager {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    config_ssh_service: ConfigSshService,

    #[inject(component)]
    port_forward_service: PortForwardService,

    #[inject(func = create_clients())]
    clients: Arc<DashMap<String, Arc<ClientForward>>>,

    client_session_factory: ComponentRef<ClientSessionFactory>,
}

#[async_trait]
impl BaseApp for ForwardManager {
    fn name(&self) -> &'static str {
        "ForwardManager"
    }
    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let manager = self.clone();
        tokio::spawn(async move {
            manager.bootstrap_enabled().await;
        });
        Ok(())
    }
}

impl ForwardManager {
    pub async fn sync_rule(&self, rule: PortForward) {
        if rule.enabled {
            self.start_rule(rule).await;
            return;
        }
        self.stop_rule(rule.id).await;
    }

    pub async fn start_rule(&self, rule: PortForward) {
        let forward_id = rule.id.clone();
        self.stop_existing(&forward_id).await;
        let context = match self.build_context(&rule).await {
            Ok(context) => context,
            Err(error) => {
                self.notify(&forward_id, FORWARD_STATUS_ERROR, RespEventForward::OnError(error.to_string()));
                return;
            }
        };
        let handler = self.make_handler(forward_id.clone());
        let client = Arc::new(ClientForward::new(forward_id.clone(), self.client_session_factory.clone()));
        match client.start(rule, context, handler).await {
            Ok(bound_port) => {
                self.clients.insert(forward_id.clone(), client);
                self.notify(&forward_id, FORWARD_STATUS_RUNNING, RespEventForward::OnStarted(bound_port));
            }
            Err(error) => {
                self.notify(&forward_id, FORWARD_STATUS_ERROR, RespEventForward::OnError(error.to_string()));
            }
        }
    }

    pub async fn stop_rule(&self, forward_id: String) {
        self.stop_existing(&forward_id).await;
        self.notify(&forward_id, FORWARD_STATUS_IDLE, RespEventForward::OnStopped);
        self.start_unstarted_enabled(Some(&forward_id)).await;
    }

    pub async fn stop_by_device_id(&self, device_id: String) {
        for rule in self.port_forward_service.select_by_device(device_id) {
            self.client_session_factory.cancel_connect(&rule.id);
            self.stop_existing(&rule.id).await;
            self.notify(&rule.id, FORWARD_STATUS_IDLE, RespEventForward::OnStopped);
        }
    }

    async fn bootstrap_enabled(&self) {
        self.start_unstarted_enabled(None).await;
    }

    async fn start_unstarted_enabled(&self, except_id: Option<&str>) {
        let rules = match self.port_forward_service.find_all().await {
            Ok(rules) => rules,
            Err(error) => {
                warn!("[ssh] forward 启动加载规则失败: {}", error);
                return;
            }
        };
        for ptr in rules {
            let rule = ptr.read().clone();
            if !rule.enabled {
                continue;
            }
            if except_id.is_some_and(|id| rule.id == id) {
                continue;
            }
            if self.clients.contains_key(&rule.id) {
                continue;
            }
            self.start_rule(rule).await;
        }
    }

    async fn build_context(&self, rule: &PortForward) -> anyhow::Result<Arc<ContextSsh>> {
        let device_id = rule.device_id.clone();
        let device = self.device_service.find_by_id(device_id.clone()).await?.ok_or_else(|| anyhow::anyhow!("device not found for device_id: {}", device_id))?;
        let config = self.config_ssh_service.get_by_device_id(device_id.clone()).ok_or_else(|| anyhow::anyhow!("ssh config not found for device_id: {}", device_id))?;
        Ok(Arc::new(ContextSsh { size: WindowSize::default(), config, device }))
    }

    async fn stop_existing(&self, forward_id: &str) {
        let Some((_, client)) = self.clients.remove(forward_id) else {
            return;
        };
        if let Err(error) = client.stop().await {
            warn!("[ssh] forward 停止隧道失败 {}: {}", forward_id, error);
        }
    }

    fn notify(&self, forward_id: &str, status: i16, event: RespEventForward) {
        self.port_forward_service.set_status(forward_id.to_string(), status);
        self.ws_manager.broadcast_all(&Self::build_resp(forward_id.to_string(), event));
    }

    fn make_handler(&self, forward_id: String) -> EventHandler<RespEventForward> {
        let ws_manager = self.ws_manager.clone();
        Arc::new(move |event: RespEventForward| {
            let ws_manager = ws_manager.clone();
            let forward_id = forward_id.clone();
            Box::pin(async move {
                ws_manager.broadcast_all(&Self::build_resp(forward_id, event));
                Ok(())
            })
        })
    }

    fn build_resp(session_id: String, event: RespEventForward) -> RespSocketContext {
        let mut resp = RespSocketContext { r#type: event.to_type(), session_id, payload: None };
        match event {
            RespEventForward::OnStarted(port) => resp.payload = Some(serde_json::to_value(port).unwrap()),
            RespEventForward::OnError(err) => resp.payload = Some(serde_json::to_value(err).unwrap()),
            RespEventForward::OnOpenStep(step) => resp.payload = Some(serde_json::to_value(step).unwrap()),
            RespEventForward::OnStopped => {}
        }
        resp
    }
}
