use parking_lot::RwLock;
use std::sync::OnceLock;
use crate::model::ai_provide;
use crate::model::dto::shared_ptr::Shared;

pub fn cloud_list() -> &'static RwLock<Vec<Shared<ai_provide::Model>>> {
    static CLOUD_LIST: OnceLock<RwLock<Vec<Shared<ai_provide::Model>>>> = OnceLock::new();
    CLOUD_LIST.get_or_init(|| RwLock::new(Vec::new()))
}

pub fn update_cloud_key(token: Option<String>) {
    for item in cloud_list().read().iter() {
        item.write().key = token.clone();
    }
}
