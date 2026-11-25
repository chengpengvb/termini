use anyhow::Result;
use tauri::AppHandle;
use summer::plugin::Service;
use tauri_plugin_notification::NotificationExt;

#[derive(Clone, Service)]
pub struct NotifyTrayService {
    #[inject(component)]
    handle: AppHandle,
}

impl NotifyTrayService {
    pub fn notify(&self, title: &str, body: &str) -> Result<()> {
        self.handle.notification().builder().title(title).body(body).show()?;
        Ok(())
    }
}
