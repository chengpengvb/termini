use serde_json::Value;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::model::job_device;
use std::collections::HashSet;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use termini_base::submit_sync_entry;
use crate::dao::dao_device::DeviceDao;
use crate::util::time_util::now_millis;
use crate::model::job_device::JobDevice;
use crate::collect::job::JobTargetService;
use crate::model::base_target::BaseTarget;
use crate::model::dto::shared_ptr::Shared;
use crate::dao::dao_job_device::JobDeviceDao;
use crate::service::service_base::BaseService;
use crate::service::service_base_sync::{self, BaseSync};
use crate::model::resp::resp_metric_item::RespMetricItem;
use crate::model::resp::resp_model::{BizError, ResultCode};
use crate::model::job_device_2_indicator::JobDevice2Indicator;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
use crate::service::service_job_indicator::JobIndicatorService;
use crate::service::service_job_indicator_alert::JobIndicatorAlertService;
use crate::service::service_job_indicator_value::JobIndicatorValueService;
use crate::service::service_job_device_2_indicator::JobDevice2IndicatorService;

#[base_service(job_device::Entity, dao = "job_device_dao", service = false)]
#[derive(Clone, Service)]
pub struct JobDeviceService {
    #[inject(component)]
    device_dao: DeviceDao,

    #[inject(component)]
    job_device_dao: JobDeviceDao,

    #[inject(component)]
    job_target_service: JobTargetService,

    #[inject(component)]
    job_indicator_service: JobIndicatorService,

    #[inject(component)]
    job_indicator_alert_service: JobIndicatorAlertService,

    #[inject(component)]
    job_indicator_value_service: JobIndicatorValueService,

    #[inject(component)]
    job_device_2_indicator_service: JobDevice2IndicatorService,
}

#[async_trait]
impl BaseService for JobDeviceService {
    type Entity = job_device::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.job_device_dao.clone()))
    }

    async fn insert(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let list = self.find_all().await?;
        if let Some(found) = list.into_iter().find(|item| item.read().device_id == entity.device_id) {
            let existing_id = found.read().id.clone();
            if !self.referenced_job_device_ids().contains(&entity.id) {
                return Err(BizError::duplicate(existing_id).into());
            }
        }
        self.get_dao()?.insert(entity).await
    }

    async fn update(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let list = self.find_all().await?;
        for found in &list {
            let row = found.read();
            if row.id != entity.id && row.device_id == entity.device_id {
                if !self.referenced_job_device_ids().contains(&entity.id) {
                    return Err(BizError::duplicate(row.id.clone()).into());
                }
            }
        }
        self.get_dao()?.update(entity).await
    }
}

impl JobDeviceService {
    pub async fn list_detail(&self) -> anyhow::Result<Vec<JobDevice>> {
        let links = self.job_device_2_indicator_service.find_all().await?;
        let mut list: Vec<JobDevice> = Vec::new();
        for shared in self.job_device_dao.find_all().await? {
            let mut job_device = shared.read().clone();
            job_device.device = self.device_dao.find_by_id(job_device.device_id.clone()).await?.map(|item| item.read().clone());
            let mut indicators: Vec<JobDevice2Indicator> = Vec::new();
            for link in &links {
                let mut link = link.read().clone();
                if link.job_device_id != job_device.id {
                    continue;
                }
                let Some(indicator) = self.job_indicator_service.find_by_id(link.job_indicator_id.clone()).await? else {
                    continue;
                };
                link.job_indicator = Some(indicator.read().clone());
                indicators.push(link);
            }
            indicators.sort_by(|left, right| Self::indicator_order(left).cmp(&Self::indicator_order(right)));
            job_device.indicators = Some(indicators);
            list.push(job_device);
        }
        list.sort_by(|left, right| left.updated_at.cmp(&right.updated_at));
        Ok(list)
    }

    pub async fn list_metric(&self, job_device_id: String, job_indicator_ids: Vec<String>) -> anyhow::Result<Vec<RespMetricItem>> {
        let mut list: Vec<RespMetricItem> = Vec::with_capacity(job_indicator_ids.len());
        for job_indicator_id in job_indicator_ids {
            list.push(self.job_indicator_value_service.build_metric(&job_device_id, &job_indicator_id).await?);
        }
        Ok(list)
    }

    pub fn take_collect_gap(&self) -> i64 {
        self.job_target_service.take_collect_gap()
    }

    pub async fn set_enabled(&self, id: String, enabled: bool) -> anyhow::Result<bool> {
        let Some(shared) = self.find_by_id(id).await? else {
            return Err(ResultCode::JobDeviceNotFound.into());
        };
        let mut job_device = shared.read().clone();
        if job_device.enabled == enabled {
            return Ok(enabled);
        }
        job_device.enabled = enabled;
        job_device.updated_at = now_millis();
        self.update(job_device).await?;
        Ok(enabled)
    }

    pub async fn save_batch(&self, device_ids: Vec<String>) -> anyhow::Result<u64> {
        let mut rows: Vec<JobDevice> = Vec::new();
        let mut exists: Vec<String> = self.job_device_dao.find_all().await?.iter().map(|item| item.read().device_id.clone()).collect();
        for device_id in device_ids {
            let device_id = device_id.trim().to_string();
            if device_id.is_empty() || exists.contains(&device_id) {
                continue;
            }
            if self.device_dao.find_by_id(device_id.clone()).await?.is_none() {
                continue;
            }
            exists.push(device_id.clone());
            rows.push(JobDevice { id: uuid(), device_id, enabled: true, updated_at: now_millis(), device: None, indicators: None });
        }
        if rows.is_empty() {
            return Ok(0);
        }
        let saved = self.insert_batch(rows).await?;
        Ok(saved.len() as u64)
    }

    pub async fn delete_detail(&self, job_device_id: String) -> anyhow::Result<u64> {
        self.cascade_delete(vec![job_device_id.clone()]).await?;
        self.delete_by_id(job_device_id).await
    }

    pub async fn delete_by_device_id(&self, device_id: String) -> anyhow::Result<u64> {
        let job_devices = self.job_device_dao.find_all().await?;
        let job_device_ids: Vec<String> = job_devices.iter().filter(|item| item.read().device_id == device_id).map(|item| item.read().id.clone()).collect();
        self.cascade_delete(job_device_ids).await?;
        self.job_device_dao.delete_by_device_id(device_id).await
    }

    async fn cascade_delete(&self, job_device_ids: Vec<String>) -> anyhow::Result<()> {
        if job_device_ids.is_empty() {
            return Ok(());
        }
        let indicator_ids = self.job_device_2_indicator_service.list_indicator_ids_by_job_device_ids(&job_device_ids);
        self.job_target_service.delete_by_job_device_ids(job_device_ids.clone()).await;
        self.job_indicator_value_service.delete_by_job_device_ids(job_device_ids.clone());
        self.job_device_2_indicator_service.delete_by_job_device_ids(job_device_ids.clone()).await?;
        self.job_indicator_service.delete_by_ids(self.list_orphan_indicator_ids(indicator_ids)).await?;
        self.job_indicator_alert_service.delete_by_job_device_ids(job_device_ids).await?;
        Ok(())
    }

    fn indicator_order(link: &JobDevice2Indicator) -> (i16, String) {
        match &link.job_indicator {
            Some(indicator) => (indicator.sort, indicator.name.clone()),
            None => (i16::MAX, String::new()),
        }
    }

    fn list_orphan_indicator_ids(&self, indicator_ids: Vec<String>) -> Vec<String> {
        let mut orphans: Vec<String> = Vec::new();
        for indicator_id in indicator_ids {
            if orphans.contains(&indicator_id) {
                continue;
            }
            if self.job_device_2_indicator_service.exists_by_job_indicator_id(&indicator_id) {
                continue;
            }
            orphans.push(indicator_id);
        }
        orphans
    }

    fn referenced_job_device_ids(&self) -> HashSet<String> {
        let mut ids = self.job_device_2_indicator_service.referenced_job_device_ids();
        ids.extend(self.job_indicator_alert_service.referenced_job_device_ids());
        ids.extend(self.job_indicator_value_service.referenced_job_device_ids());
        ids
    }
}

#[async_trait]
impl BaseSync for JobDeviceService {
    fn sync_type(&self) -> i8 {
        9
    }

    fn merged(&self, merged: Vec<Value>, cloud: &[Value], local: &[Value], op: i8) -> anyhow::Result<Vec<Value>> {
        let _ = (cloud, local, op);
        service_base_sync::keep_referenced(merged, "device_id", &self.referenced_job_device_ids())
    }

    async fn export_current(&self) -> anyhow::Result<Value> {
        service_base_sync::export_current(self).await
    }

    async fn import_cloud(&self, data: Value) -> anyhow::Result<()> {
        service_base_sync::import_cloud(self, data, None).await
    }
}

submit_sync_entry!(JobDeviceService);
