use anyhow::Result;
use summer::plugin::Service;
use crate::model::job_discover;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use crate::util::time_util::now_millis;
use crate::web::ws::ws_manager::WsManager;
use crate::service::service_base::BaseService;
use crate::dao::dao_job_discover::JobDiscoverDao;
use crate::service::service_ai_chat::AiChatService;
use crate::dao::dao_job_discover_item::JobDiscoverItemDao;
use crate::model::req::req_job_indicator::ReqJobIndicator;
use crate::service::service_job_indicator::JobIndicatorService;
use crate::model::job_discover::{JobDiscover, JobDiscoverStatus};
use crate::model::job_indicator::{JobIndicator, JobIndicatorAlertAiOn};
use crate::model::resp::event::resp_event_job_discover::RespEventJobDiscover;
use crate::model::job_discover_item::{JobDiscoverItem, JobDiscoverItemStatus};

#[base_service(job_discover::Entity, dao = "job_discover_dao")]
#[derive(Clone, Service)]
pub struct JobDiscoverService {
    #[inject(component)]
    ws_manager: WsManager,

    #[inject(component)]
    ai_chat_service: AiChatService,

    #[inject(component)]
    job_discover_dao: JobDiscoverDao,

    #[inject(component)]
    job_discover_item_dao: JobDiscoverItemDao,

    #[inject(component)]
    job_indicator_service: JobIndicatorService,
}

impl JobDiscoverService {
    pub async fn create_run(&self, chat_id: String, job_device_id: String, ai_provide_id: Option<String>) -> Result<JobDiscover> {
        let run = JobDiscover { id: uuid(), chat_id, job_device_id, status: JobDiscoverStatus::Queued as i16, timestamp: now_millis(), ended_at: None, summary: None, error: None, ai_provide_id };
        let saved = self.insert(run).await?;
        let run = saved.read().clone();
        self.emit_changed(&run.id);
        Ok(run)
    }

    pub async fn mark_running(&self, chat_id: &str) -> Result<()> {
        self.update_status(chat_id, JobDiscoverStatus::Running, None, None).await
    }

    pub async fn mark_result(&self, chat_id: &str, status: JobDiscoverStatus, summary: Option<String>, error: Option<String>) -> Result<()> {
        self.update_status(chat_id, status, summary, error).await
    }

    async fn update_status(&self, chat_id: &str, status: JobDiscoverStatus, summary: Option<String>, error: Option<String>) -> Result<()> {
        let Some(mut run) = self.job_discover_dao.find_by_chat_id(chat_id) else {
            return Ok(());
        };
        run.status = status as i16;
        if summary.is_some() {
            run.summary = summary;
        }
        run.error = error;
        if !status.is_running() {
            run.ended_at = Some(now_millis());
        }
        let discover_id = run.id.clone();
        self.update(run).await?;
        self.emit_changed(&discover_id);
        Ok(())
    }

    pub async fn add_candidates(&self, items: Vec<JobDiscoverItem>) -> Result<usize> {
        let mut count = 0;
        for mut item in items {
            item.id = uuid();
            item.timestamp = now_millis();
            item.status = JobDiscoverItemStatus::Candidate as i16;
            self.job_discover_item_dao.insert(item).await?;
            count += 1;
        }
        Ok(count)
    }

    pub fn list_items(&self, discover_id: &str) -> Vec<JobDiscoverItem> {
        self.job_discover_item_dao.list_by_discover_id(discover_id)
    }

    pub async fn accept(&self, ids: Vec<String>) -> Result<()> {
        for id in ids {
            let Some(shared) = self.job_discover_item_dao.find_by_id(id.clone()).await? else {
                continue;
            };
            let mut item = shared.read().clone();
            if item.status != JobDiscoverItemStatus::Candidate as i16 {
                continue;
            }
            let indicator = Self::build_indicator(&item);
            let req = ReqJobIndicator { interval_secs: item.interval_secs, job_device_id: item.job_device_id.clone(), job_indicator: indicator };
            let link = self.job_indicator_service.save_with_job(req).await?;
            item.status = JobDiscoverItemStatus::Accepted as i16;
            item.job_indicator_id = Some(link.job_indicator_id.clone());
            self.job_discover_item_dao.update(item).await?;
        }
        Ok(())
    }

    pub async fn delete_by_ids(&self, ids: Vec<String>) -> Result<Vec<JobDiscover>> {
        let mut removed = Vec::with_capacity(ids.len());
        for id in &ids {
            if let Some(shared) = self.find_by_id(id.clone()).await? {
                removed.push(shared.read().clone());
            }
        }
        if removed.is_empty() {
            return Ok(removed);
        }
        let discover_ids: Vec<String> = removed.iter().map(|item| item.id.clone()).collect();
        let chat_ids: Vec<String> = removed.iter().map(|item| item.chat_id.clone()).filter(|id| !id.is_empty()).collect();
        self.job_discover_item_dao.delete_by_discover_ids(discover_ids.clone()).await?;
        self.job_discover_dao.delete_by_ids(discover_ids.clone()).await?;
        for chat_id in chat_ids {
            self.ai_chat_service.delete_by_id(chat_id).await?;
        }
        for discover_id in &discover_ids {
            self.emit_changed(discover_id);
        }
        Ok(removed)
    }

    fn build_indicator(item: &JobDiscoverItem) -> JobIndicator {
        let alert_ai_on = JobIndicatorAlertAiOn::Off as i16;
        JobIndicator {
            sort: 0,
            alert_ai_on,
            enabled: true,
            id: String::new(),
            is_default: false,
            alert_sensitivity: 0,
            alert_enabled: false,
            arg: item.arg.clone(),
            name: item.name.clone(),
            protocol: item.protocol,
            alert_ai_repair_message: None,
            parser_type: item.parser_type,
            range_min: item.range_min,
            range_max: item.range_max,
            display_unit: item.display_unit,
            alert_rule_op: item.alert_rule_op,
            parse_expr: item.parse_expr.clone(),
            transform_expr: item.transform_expr.clone(),
            alert_threshold: item.alert_threshold.clone(),
        }
    }

    fn emit_changed(&self, discover_id: &str) {
        self.ws_manager.broadcast_all(&RespEventJobDiscover::OnChanged(discover_id.to_string()).to_resp());
    }
}
