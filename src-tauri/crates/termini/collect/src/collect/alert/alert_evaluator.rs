use std::sync::Arc;
use dashmap::DashMap;
use sea_orm::JsonValue;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use crate::collect::alert::alert_rule;
use crate::module::common::alert_hook;
use crate::util::time_util::now_millis;
use crate::collect::dto::CollectContext;
use summer::tracing::{debug, info, warn};
use crate::service::service_base::BaseService;
use crate::base::base_app::{AppContext, BaseApp};
use crate::dao::dao_job_indicator::JobIndicatorDao;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::service::service_job_indicator_alert::JobIndicatorAlertService;
use crate::service::service_job_indicator_value::JobIndicatorValueService;
use crate::model::job_indicator::{JobIndicatorAlertAiOn, JobIndicatorAlertRuleOp};
use crate::model::job_indicator_alert::{AlertStatus, FireType, JobIndicatorAlert, ResolveType};

pub const SENSITIVITY_MIN: i32 = 1;

#[derive(Clone, Eq, Hash, PartialEq)]
struct AlertKey {
    job_device_id: String,
    job_indicator_id: String,
}

#[derive(Default)]
struct AlertState {
    hit: i32,
    miss: i32,
    firing_id: Option<String>,
}

enum AlertAction {
    Idle,
    Fire,
    Resolve(String),
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum AlertHit {
    Miss,
    CollectFailed,
    OverThreshold,
}

fn create_states() -> Arc<DashMap<AlertKey, AlertState>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 254)]
#[derive(Clone, Service)]
pub struct AlertEvaluator {
    #[inject(component)]
    job_indicator_dao: JobIndicatorDao,

    #[inject(func = create_states())]
    states: Arc<DashMap<AlertKey, AlertState>>,

    #[inject(component)]
    job_indicator_alert_service: JobIndicatorAlertService,

    #[inject(component)]
    job_indicator_value_service: JobIndicatorValueService,
}

#[async_trait]
impl BaseApp for AlertEvaluator {
    fn name(&self) -> &'static str {
        "AlertEvaluator"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.restore_firing().await;
        Ok(())
    }
}

impl AlertEvaluator {
    pub async fn evaluate_all(&self, ctx: &CollectContext, values: &[JobIndicatorValue]) -> anyhow::Result<()> {
        if values.is_empty() {
            return Ok(());
        }
        let (alert_enabled, rule_op, threshold, sensitivity) = {
            let indicator = ctx.job_indicator.read();
            (indicator.alert_enabled, indicator.alert_rule_op, indicator.alert_threshold.clone(), indicator.alert_sensitivity)
        };
        if !alert_enabled {
            return Ok(());
        }
        let Some(op) = JobIndicatorAlertRuleOp::from_i16(rule_op) else {
            return Ok(());
        };
        if threshold.trim().is_empty() {
            return Ok(());
        }
        let sensitivity = sensitivity.max(SENSITIVITY_MIN);
        for value in values {
            self.evaluate(ctx, value, op, &threshold, sensitivity).await?;
        }
        Ok(())
    }

    async fn evaluate(&self, ctx: &CollectContext, value: &JobIndicatorValue, op: JobIndicatorAlertRuleOp, threshold: &str, sensitivity: i32) -> anyhow::Result<()> {
        let reserved_id = uuid();
        let hit = Self::check_hit(op, value, threshold);
        let key = AlertKey { job_device_id: value.job_device_id.clone(), job_indicator_id: value.job_indicator_id.clone() };
        match self.advance(&key, hit != AlertHit::Miss, sensitivity, &reserved_id) {
            AlertAction::Idle => {}
            AlertAction::Fire => {
                let fire_type = match hit {
                    AlertHit::Miss => FireType::Rule,
                    AlertHit::OverThreshold => FireType::Rule,
                    AlertHit::CollectFailed => FireType::CollectFailed,
                };
                let alert_id = match self.fire(&reserved_id, value, op, threshold, fire_type).await {
                    Ok(alert_id) => alert_id,
                    Err(error) => {
                        self.mark_firing(&key, None);
                        return Err(error);
                    }
                };
                self.mark_firing(&key, Some(alert_id.clone()));
                if hit == AlertHit::OverThreshold {
                    self.spawn_diagnose(ctx, alert_id.clone());
                }
                debug!("[collect] fire job_device={} job_indicator={} alert={}", key.job_device_id, key.job_indicator_id, alert_id);
            }
            AlertAction::Resolve(alert_id) => {
                self.resolve(&alert_id, ResolveType::Rule, value.value.clone()).await?;
                self.mark_firing(&key, None);
                debug!("[collect] resolve job_device={} job_indicator={} alert={}", key.job_device_id, key.job_indicator_id, alert_id);
            }
        }
        Ok(())
    }

    fn spawn_diagnose(&self, ctx: &CollectContext, alert_id: String) {
        if JobIndicatorAlertAiOn::from_i16(ctx.job_indicator.read().alert_ai_on).is_none() {
            return;
        }
        alert_hook::enqueue_alert(alert_id);
    }

    fn check_hit(op: JobIndicatorAlertRuleOp, value: &JobIndicatorValue, threshold: &str) -> AlertHit {
        if !value.success {
            return AlertHit::CollectFailed;
        }
        let Some(raw) = value.value.as_deref() else {
            return AlertHit::CollectFailed;
        };
        if alert_rule::is_hit(op, raw, threshold) {
            return AlertHit::OverThreshold;
        }
        AlertHit::Miss
    }

    fn advance(&self, key: &AlertKey, hit: bool, sensitivity: i32, reserved_id: &str) -> AlertAction {
        let mut state = self.states.entry(key.clone()).or_default();
        if hit {
            state.miss = 0;
            state.hit = state.hit.saturating_add(1);
            if state.firing_id.is_none() && state.hit >= sensitivity {
                state.hit = 0;
                state.firing_id = Some(reserved_id.to_string());
                return AlertAction::Fire;
            }
            return AlertAction::Idle;
        }
        state.hit = 0;
        state.miss = state.miss.saturating_add(1);
        if state.miss < sensitivity {
            return AlertAction::Idle;
        }
        match state.firing_id.clone() {
            Some(alert_id) => AlertAction::Resolve(alert_id),
            None => AlertAction::Idle,
        }
    }

    pub async fn stop_alert(&self, job_indicator_id: &str) -> anyhow::Result<()> {
        for shared in self.job_indicator_alert_service.find_all().await? {
            let (alert_id, job_device_id) = {
                let alert = shared.read();
                if alert.status != AlertStatus::Firing as i16 || alert.job_indicator_id != job_indicator_id {
                    continue;
                }
                (alert.id.clone(), alert.job_device_id.clone())
            };
            self.resolve(&alert_id, ResolveType::AlertOff, None).await?;
            self.forget(&job_device_id, job_indicator_id);
            debug!("[collect] stop job_device={} job_indicator={} alert={}", job_device_id, job_indicator_id, alert_id);
        }
        Ok(())
    }

    pub fn forget(&self, job_device_id: &str, job_indicator_id: &str) {
        self.states.remove(&AlertKey { job_device_id: job_device_id.to_string(), job_indicator_id: job_indicator_id.to_string() });
    }

    fn mark_firing(&self, key: &AlertKey, alert_id: Option<String>) {
        let mut state = self.states.entry(key.clone()).or_default();
        state.hit = 0;
        state.miss = 0;
        state.firing_id = alert_id;
    }

    async fn fire(&self, reserved_id: &str, value: &JobIndicatorValue, op: JobIndicatorAlertRuleOp, threshold: &str, fire_type: FireType) -> anyhow::Result<String> {
        if let Some(firing) = self.job_indicator_alert_service.find_firing(&value.job_device_id, &value.job_indicator_id).await? {
            return Ok(firing.id);
        }
        let id = reserved_id.to_string();
        let alert_at = now_millis();
        let alert_rule_op = op as i16;
        let fire_type = Some(fire_type as i16);
        let status = AlertStatus::Firing as i16;
        let job_indicator_value = JsonValue::Null;
        let alert_threshold = threshold.to_string();
        let job_device_id = value.job_device_id.clone();
        let job_indicator_id = value.job_indicator_id.clone();
        let alert_value = value.value.clone().unwrap_or_default();
        let mut alert = JobIndicatorAlert {
            id,
            status,
            fire_type,
            alert_at,
            alert_rule_op,
            alert_value,
            job_device_id,
            alert_threshold,
            resolved_at: None,
            resolved_type: None,
            resolved_value: None,
            ai_summary: None,
            ai_chat_id: None,
            job_indicator_id,
            job_indicator_value,
            job_indicator: None,
        };
        alert.set_job_indicator_value(Some(value.clone()))?;
        let saved = self.job_indicator_alert_service.insert_and_notify(alert).await?;
        let alert_id = saved.read().id.clone();
        Ok(alert_id)
    }

    async fn resolve(&self, alert_id: &str, resolve_type: ResolveType, resolved_value: Option<String>) -> anyhow::Result<()> {
        let Some(shared) = self.job_indicator_alert_service.find_by_id(alert_id.to_string()).await? else {
            return Ok(());
        };
        let mut alert = shared.read().clone();
        alert.status = AlertStatus::Normal as i16;
        alert.resolved_at = Some(now_millis());
        alert.resolved_type = Some(resolve_type as i16);
        alert.resolved_value = match resolve_type {
            ResolveType::Rule => resolved_value,
            ResolveType::AlertOff => None,
        };
        let job_device_id = alert.job_device_id.clone();
        let job_indicator_id = alert.job_indicator_id.clone();
        self.job_indicator_alert_service.update(alert).await?;
        self.job_indicator_value_service.notify_metric(&job_device_id, &job_indicator_id).await?;
        Ok(())
    }

    async fn restore_firing(&self) {
        let alerts = match self.job_indicator_alert_service.find_all().await {
            Ok(alerts) => alerts,
            Err(error) => {
                warn!("[collect] restore firing failed: {}", error);
                return;
            }
        };
        let mut restored = 0usize;
        let mut stopped = 0usize;
        for shared in alerts {
            let (alert_id, key) = {
                let alert = shared.read();
                if alert.status != AlertStatus::Firing as i16 {
                    continue;
                }
                (alert.id.clone(), AlertKey { job_device_id: alert.job_device_id.clone(), job_indicator_id: alert.job_indicator_id.clone() })
            };
            if !self.alert_enabled(&key.job_indicator_id).await {
                match self.resolve(&alert_id, ResolveType::AlertOff, None).await {
                    Ok(()) => stopped += 1,
                    Err(error) => warn!("[collect] restore stop alert={} failed: {}", alert_id, error),
                }
                continue;
            }
            self.states.insert(key, AlertState { hit: 0, miss: 0, firing_id: Some(alert_id) });
            restored += 1;
        }
        info!("[collect] restore firing alerts {} stopped {}", restored, stopped);
    }

    async fn alert_enabled(&self, job_indicator_id: &str) -> bool {
        self.job_indicator_dao.find_by_id(job_indicator_id.to_string()).await.ok().flatten().map(|item| item.read().alert_enabled).unwrap_or(false)
    }
}
