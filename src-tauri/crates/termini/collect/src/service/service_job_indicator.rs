use summer::plugin::Service;
use async_trait::async_trait;
use crate::model::job_indicator;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use crate::collect::alert::AlertEvaluator;
use crate::collect::job::JobTargetService;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultCode;
use crate::dao::dao_job_indicator::JobIndicatorDao;
use crate::model::job_indicator::JobIndicatorDisplayUnit;
use crate::model::req::req_job_indicator::ReqJobIndicator;
use crate::model::job_device_2_indicator::JobDevice2Indicator;
use crate::collect::collector::r#impl::parser::parser_transform;
use crate::service::service_job_indicator_value::JobIndicatorValueService;
use crate::service::service_job_device_2_indicator::JobDevice2IndicatorService;

#[base_service(job_indicator::Entity, dao = "job_indicator_dao", sync = 10)]
#[derive(Clone, Service)]
pub struct JobIndicatorService {
    #[inject(component)]
    job_indicator_dao: JobIndicatorDao,

    #[inject(component)]
    alert_evaluator: AlertEvaluator,

    #[inject(component)]
    job_target_service: JobTargetService,

    #[inject(component)]
    job_indicator_value_service: JobIndicatorValueService,

    #[inject(component)]
    job_device_2_indicator_service: JobDevice2IndicatorService,
}

#[async_trait]
impl BaseService for JobIndicatorService {
    type Entity = job_indicator::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.job_indicator_dao.clone()))
    }
}

impl JobIndicatorService {
    pub async fn save_with_job(&self, req: ReqJobIndicator) -> anyhow::Result<JobDevice2Indicator> {
        if req.job_device_id.trim().is_empty() {
            return Err(ResultCode::IndicatorDeviceRequired.into());
        }
        let mut indicator = req.job_indicator;
        indicator.name = indicator.name.trim().to_string();
        if indicator.name.is_empty() {
            return Err(ResultCode::IndicatorNameEmpty.into());
        }
        indicator.transform_expr = indicator.transform_expr.trim().to_string();
        if parser_transform::check(&indicator.transform_expr).is_err() {
            return Err(ResultCode::IndicatorTransformInvalid.into());
        }
        indicator.alert_ai_repair_message = indicator.alert_ai_repair_message.map(|text| text.trim().to_string()).filter(|text| !text.is_empty());
        if JobIndicatorDisplayUnit::from_i16(indicator.display_unit).is_none() {
            indicator.display_unit = JobIndicatorDisplayUnit::None as i16;
        }
        let saved = if indicator.id.trim().is_empty() {
            indicator.id = uuid();
            self.insert(indicator).await?
        } else {
            self.update(indicator).await?
        };
        let job_indicator_id = saved.read().id.clone();
        if !saved.read().alert_enabled {
            self.alert_evaluator.stop_alert(&job_indicator_id).await?;
        }
        let mut link = self.job_device_2_indicator_service.save_link(&req.job_device_id, &job_indicator_id, req.interval_secs).await?;
        self.job_target_service.start_job_device(&req.job_device_id).await;
        link.job_indicator = Some(saved.read().clone());
        Ok(link)
    }

    pub async fn delete_with_job(&self, job_id: String) -> anyhow::Result<()> {
        let Some(shared) = self.job_device_2_indicator_service.find_by_id(job_id.clone()).await? else {
            return Err(ResultCode::IndicatorDeleted.into());
        };
        let link = shared.read().clone();
        self.job_device_2_indicator_service.delete_by_id(job_id).await?;
        self.job_indicator_value_service.delete_by_indicator(&link.job_device_id, &link.job_indicator_id);
        if !self.job_device_2_indicator_service.exists_by_job_indicator_id(&link.job_indicator_id) {
            self.delete_by_ids(vec![link.job_indicator_id]).await?;
        }
        self.job_target_service.start_job_device(&link.job_device_id).await;
        Ok(())
    }

    pub async fn delete_by_ids(&self, ids: Vec<String>) -> anyhow::Result<u64> {
        let mut removable = Vec::new();
        for id in ids {
            if self.is_default(&id).await {
                continue;
            }
            removable.push(id);
        }
        self.job_indicator_dao.delete_by_ids(removable).await
    }

    pub async fn is_default(&self, id: &str) -> bool {
        self.job_indicator_dao.find_by_id(id.to_string()).await.ok().flatten().map(|item| item.read().is_default).unwrap_or(false)
    }

    pub async fn select_default(&self) -> anyhow::Result<Vec<crate::model::job_indicator::JobIndicator>> {
        let all = self.find_all().await?;
        let mut defaults: Vec<crate::model::job_indicator::JobIndicator> = all.iter().filter(|item| item.read().is_default).map(|item| item.read().clone()).collect();
        defaults.sort_by_key(|item| item.sort);
        Ok(defaults)
    }
}
