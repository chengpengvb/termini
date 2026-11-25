use dashmap::DashMap;
use parking_lot::RwLock;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use std::sync::{Arc, OnceLock};
use summer::tracing::log::warn;
use crate::base::base_app::BaseApp;
use summer::tracing::{error, info};
use crate::base::base_app::AppContext;
use summer::plugin::ComponentRegistry;
use crate::web::ws::handle::handle_base::HandleBase;
use crate::model::req::req_ws_context::ReqSocketContext;
use summer_web::socketioxide::extract::{Data, SocketRef};
use crate::model::resp::resp_ws_context::RespSocketContext;
use crate::model::resp::resp_ws_bin_context::RespSocketBinContext;
use summer_web::{on_connection, on_disconnect, subscribe_message};

static MSG: &'static str = "msg";
static BUS: OnceLock<WsManager> = OnceLock::new();

pub const ROOM_ALL: &str = "__all__";

#[base_app(sort = 200)]
#[derive(Clone, Service)]
pub struct WsManager {
    #[inject(func = create_handles())]
    handles: Arc<RwLock<Vec<Arc<dyn HandleBase>>>>,

    #[inject(func = create_rooms())]
    rooms: Arc<DashMap<String, DashMap<String, SocketRef>>>,
}

fn create_handles() -> Arc<RwLock<Vec<Arc<dyn HandleBase>>>> {
    Arc::new(RwLock::new(Vec::new()))
}

fn create_rooms() -> Arc<DashMap<String, DashMap<String, SocketRef>>> {
    Arc::new(DashMap::new())
}

#[async_trait]
impl BaseApp for WsManager {
    fn name(&self) -> &'static str {
        "WsDistribute"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        let _ = BUS.set(self.clone());
        Ok(())
    }
}

impl WsManager {
    pub fn current() -> Option<&'static WsManager> {
        BUS.get()
    }

    pub fn register(&self, handle: Arc<dyn HandleBase>) {
        let name = handle.get_name();
        self.handles.write().push(handle);
        info!("[ws] 注册 handle [{}].", name);
    }

    pub fn send_msg(&self, socket: SocketRef, resp: &RespSocketContext) {
        if let Err(error) = socket.emit(MSG, resp) {
            warn!("[ws] 推送 消息失败 id={} type={}: {}", resp.session_id.clone(), resp.r#type, error);
        }
    }

    pub fn send_bin(&self, socket: SocketRef, resp: &RespSocketBinContext) {
        if let Err(error) = socket.emit(MSG, resp) {
            warn!("[ws] 推送 二进制消息失败 id={} type={}: {}", resp.session_id.clone(), resp.r#type, error);
        }
    }

    pub fn broadcast(&self, room: &str, resp: &RespSocketContext) {
        let Some(members) = self.rooms.get(room) else {
            return;
        };
        for member in members.iter() {
            if let Err(error) = member.value().emit(MSG, resp) {
                warn!("[ws] 房间推送失败 room={} type={}: {}", room, resp.r#type, error);
            }
        }
    }

    pub fn broadcast_all(&self, resp: &RespSocketContext) {
        self.broadcast(ROOM_ALL, resp);
    }

    pub fn join(&self, room: &str, socket: SocketRef) {
        let members = self.rooms.entry(room.to_string()).or_default();
        members.insert(socket.id.to_string(), socket);
        info!("[ws] 进入房间 room={} 在线={}", room, members.len());
    }

    pub fn leave(&self, room: &str, socket_id: &str) {
        let Some(members) = self.rooms.get(room) else {
            return;
        };
        let removed = members.remove(socket_id).is_some();
        let online = members.len();
        drop(members);
        self.rooms.remove_if(room, |_, members| members.is_empty());
        if removed {
            info!("[ws] 离开房间 room={} 在线={}", room, online);
        }
    }

    fn leave_all(&self, socket_id: &str) {
        let rooms: Vec<String> = self.rooms.iter().map(|entry| entry.key().clone()).collect();
        for room in rooms {
            self.leave(&room, socket_id);
        }
    }

    fn snapshot_handles(&self) -> Vec<Arc<dyn HandleBase>> {
        self.handles.read().clone()
    }

    async fn handle_connection(&self, socket: SocketRef) {
        let handles = self.snapshot_handles();
        if handles.is_empty() {
            error!("[ws] handles is empty, ignore connection sid={:?}", socket.id);
            return;
        }
        info!("[ws] 客户端已连接 sid={:?}", socket.id);
        self.join(ROOM_ALL, socket.clone());
        for h in handles {
            if let Err(e) = h.handle_open(socket.clone()).await {
                let name = h.get_name();
                error!("[ws] 连接回调失败 name={name} error={e:?}");
            }
        }
    }

    async fn handle_msg(&self, socket: SocketRef, req: Arc<ReqSocketContext>) {
        let handles = self.snapshot_handles();
        if handles.is_empty() {
            error!("[ws] handles is empty, ignore msg sid={:?} type={}", socket.id, req.r#type);
            return;
        }
        let mut handled = false;
        let r#type = req.r#type;
        for handle in handles {
            if !handle.contains_type(r#type) {
                continue;
            }
            handled = true;
            if let Err(e) = handle.handle_msg(socket.clone(), Arc::clone(&req)).await {
                let name = handle.get_name();
                error!("[ws] 消息处理失败 name={name} error={e:?}");
            }
        }
        if !handled {
            warn!("[ws] 未找到对应类型的 handle type={}", r#type);
        }
    }

    async fn handle_disconnect(&self, socket: SocketRef) {
        self.leave_all(&socket.id.to_string());
        let handles = self.snapshot_handles();
        if handles.is_empty() {
            error!("[ws] handles is empty, ignore disconnect sid={:?}", socket.id);
            return;
        }
        info!("[ws] 客户端已断开 sid={:?}", socket.id);
        for h in handles {
            if let Err(e) = h.handle_close(socket.clone()).await {
                let name = h.get_name();
                error!("[ws] handle 断开回调失败 name={name} error={e:?}");
            }
        }
    }
}

fn bus() -> &'static WsManager {
    BUS.get_or_init(|| {
        let app = summer::App::global();
        app.get_expect_component::<WsManager>()
    })
}

#[subscribe_message("msg")]
async fn on_msg(socket: SocketRef, Data(req): Data<ReqSocketContext>) {
    bus().handle_msg(socket, Arc::new(req)).await;
}

#[on_connection]
async fn on_connection(socket: SocketRef) {
    bus().handle_connection(socket).await;
}

#[on_disconnect]
async fn on_disconnect(socket: SocketRef) {
    bus().handle_disconnect(socket).await;
}
