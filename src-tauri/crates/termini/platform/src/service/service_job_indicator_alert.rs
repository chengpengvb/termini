use summer::tracing::warn;
use summer::plugin::Service;
use std::collections::HashSet;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::dao::dao_device::DeviceDao;
use crate::model::job_indicator_alert;
use crate::model::dto::shared_ptr::Shared;
use crate::web::ws::ws_manager::WsManager;
use crate::dao::dao_job_device::JobDeviceDao;
use crate::service::service_base::BaseService;
use crate::service::service_config::ConfigService;
use crate::dao::dao_job_indicator::JobIndicatorDao;
use crate::service::service_notify_mail::NotifyMailService;
use crate::service::service_notify_tray::NotifyTrayService;
use crate::dao::dao_job_indicator_alert::JobIndicatorAlertDao;
use crate::service::service_notify_webhook::NotifyWebhookService;
use crate::model::resp::event::resp_event_job_alert::RespEventJobAlert;
use crate::model::job_indicator_alert::{AlertStatus, JobIndicatorAlert};

#[base_service(job_indicator_alert::Entity, dao = "job_indicator_alert_dao")]
#[derive(Clone, Service)]
pub struct JobIndicatorAlertService {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    device_dao: DeviceDao,

    #[inject(component)]
    job_device_dao: JobDeviceDao,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    job_indicator_dao: JobIndicatorDao,

    #[inject(component)]
    notify_mail_service: NotifyMailService,

    #[inject(component)]
    notify_tray_service: NotifyTrayService,

    #[inject(component)]
    notify_webhook_service: NotifyWebhookService,

    #[inject(component)]
    job_indicator_alert_dao: JobIndicatorAlertDao,
}

impl JobIndicatorAlertService {
    pub async fn list_by_device_indicator(&self, job_device_id: &str, job_indicator_id: &str) -> anyhow::Result<Vec<JobIndicatorAlert>> {
        let alerts = self.find_all().await?;
        let mut list: Vec<JobIndicatorAlert> = Vec::new();
        for shared in alerts {
            let mut alert = shared.read().clone();
            if alert.job_device_id != job_device_id || alert.job_indicator_id != job_indicator_id {
                continue;
            }
            alert.job_indicator = self.job_indicator_dao.find_by_id(alert.job_indicator_id.clone()).await?.map(|item| item.read().clone());
            list.push(alert);
        }
        list.sort_by(|left, right| right.alert_at.cmp(&left.alert_at));
        Ok(list)
    }

    pub async fn list_recent(&self, limit: usize) -> anyhow::Result<Vec<JobIndicatorAlert>> {
        let alerts = self.find_all().await?;
        let mut list: Vec<JobIndicatorAlert> = alerts.iter().map(|shared| shared.read().clone()).collect();
        list.sort_by(|left, right| right.alert_at.cmp(&left.alert_at));
        list.truncate(limit);
        for alert in list.iter_mut() {
            alert.job_indicator = self.job_indicator_dao.find_by_id(alert.job_indicator_id.clone()).await?.map(|item| item.read().clone());
        }
        Ok(list)
    }

    pub async fn find_firing(&self, job_device_id: &str, job_indicator_id: &str) -> anyhow::Result<Option<JobIndicatorAlert>> {
        for shared in self.find_all().await? {
            let alert = shared.read();
            if alert.status != AlertStatus::Firing as i16 {
                continue;
            }
            if alert.job_device_id == job_device_id && alert.job_indicator_id == job_indicator_id {
                return Ok(Some(alert.clone()));
            }
        }
        Ok(None)
    }

    pub async fn delete_by_ids(&self, ids: Vec<String>) -> anyhow::Result<Vec<JobIndicatorAlert>> {
        let mut removed = Vec::with_capacity(ids.len());
        for id in &ids {
            if let Some(shared) = self.find_by_id(id.clone()).await? {
                removed.push(shared.read().clone());
            }
        }
        self.job_indicator_alert_dao.delete_by_ids(ids.clone()).await?;
        for id in &ids {
            self.emit_changed(id);
        }
        Ok(removed)
    }

    pub async fn delete_by_job_device_ids(&self, job_device_ids: Vec<String>) -> anyhow::Result<u64> {
        self.job_indicator_alert_dao.delete_by_job_device_ids(job_device_ids).await
    }

    pub fn referenced_job_device_ids(&self) -> HashSet<String> {
        self.job_indicator_alert_dao.cache_map().iter().map(|entry| entry.value().read().job_device_id.clone()).collect()
    }

    pub async fn insert_and_notify(&self, alert: JobIndicatorAlert) -> anyhow::Result<Shared<JobIndicatorAlert>> {
        let saved = self.insert(alert.clone()).await?;
        self.emit_changed(&saved.read().id);
        self.notify_fire(&alert).await;
        Ok(saved)
    }

    pub async fn update(&self, entity: JobIndicatorAlert) -> anyhow::Result<Shared<JobIndicatorAlert>> {
        let saved = self.job_indicator_alert_dao.update(entity).await?;
        self.emit_changed(&saved.read().id);
        Ok(saved)
    }

    fn emit_changed(&self, alert_id: &str) {
        self.ws_manager.broadcast_all(&RespEventJobAlert::OnChanged(alert_id.to_string()).to_resp());
    }

    async fn notify_fire(&self, alert: &JobIndicatorAlert) {
        let Some(config) = self.config_service.select_config().await else {
            return;
        };
        let mail_target = if config.notify_enabled { config.notify_email.clone().filter(|item| !item.trim().is_empty()) } else { None };
        let webhook_target = if config.notify_webhook_enabled { config.notify_webhook_url.clone().filter(|item| !item.trim().is_empty()) } else { None };
        if !config.notify_tray_enabled && mail_target.is_none() && webhook_target.is_none() {
            return;
        }
        let (device_name, indicator_name) = match self.resolve_names(alert).await {
            Ok(names) => names,
            Err(error) => {
                warn!("[alert] resolve names failed alert={}: {:#}", alert.id, error);
                return;
            }
        };
        let subject = format!("[Alert] {} - {}", device_name, indicator_name);
        let body = format!("Device: {}\nIndicator: {}\nValue: {}\nThreshold: {}\nFired at (ms): {}", device_name, indicator_name, alert.alert_value, alert.alert_threshold, alert.alert_at);
        if config.notify_tray_enabled {
            if let Err(error) = self.notify_tray_service.notify(&subject, &body) {
                warn!("[alert] tray notify failed alert={}: {:#}", alert.id, error);
            }
        }
        if let Some(email) = mail_target {
            if let Err(error) = self.notify_mail_service.send_mail(std::slice::from_ref(&email), &subject, &body).await {
                warn!("[alert] send mail failed alert={}: {:#}", alert.id, error);
            }
        }
        if let Some(url) = webhook_target {
            let text = format!("{}\n{}", subject, body);
            if let Err(error) = self.notify_webhook_service.send(config.notify_webhook_kind, &url, config.notify_webhook_extra.as_deref(), &text).await {
                warn!("[alert] send webhook failed alert={}: {:#}", alert.id, error);
            }
        }
    }

    async fn resolve_names(&self, alert: &JobIndicatorAlert) -> anyhow::Result<(String, String)> {
        let device_name = match self.job_device_dao.find_by_id(alert.job_device_id.clone()).await? {
            None => "Unknown device".to_string(),
            Some(job_device) => {
                let device_id = job_device.read().device_id.clone();
                match self.device_dao.find_by_id(device_id).await? {
                    None => "Unknown device".to_string(),
                    Some(device) => device.read().name.clone(),
                }
            }
        };
        let indicator_name = match self.job_indicator_dao.find_by_id(alert.job_indicator_id.clone()).await? {
            None => "Unknown indicator".to_string(),
            Some(indicator) => indicator.read().name.clone(),
        };
        Ok((device_name, indicator_name))
    }
}
