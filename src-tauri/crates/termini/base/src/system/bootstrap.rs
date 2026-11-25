use summer::App;
use std::sync::Arc;
use crate::base::base_app::BaseApp;
use crate::service::service_ai_crud;
use crate::service::service_base_sync;
use crate::system::system_app::SystemApp;

pub struct BootstrapEntry {
    pub sort: i16,
    pub fetch: fn(&App) -> Option<Arc<dyn BaseApp>>,
}

inventory::collect!(BootstrapEntry);

pub fn wire_all(app: &App, system: &SystemApp) {
    for reg in inventory::iter::<BootstrapEntry> {
        if let Some(arc) = (reg.fetch)(app) {
            system.register(arc, reg.sort);
        }
    }
    service_base_sync::wire(app);
    service_ai_crud::wire(app);
}
