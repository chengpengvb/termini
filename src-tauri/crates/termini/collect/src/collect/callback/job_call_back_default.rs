use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::BaseApp;
use summer::tracing::{debug, warn};
use crate::collect::dto::CollectContext;
use crate::collect::alert::AlertEvaluator;
use crate::collect::callback::JobCallBack;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::service::service_log_throttle::LogThrottleService;
use crate::service::service_job_indicator_value::JobIndicatorValueService;

#[base_app(sort = 249)]
#[derive(Clone, Service)]
pub struct DefaultJobCallBack {
    #[inject(component)]
    alert_evaluator: AlertEvaluator,

    #[inject(component)]
    log_throttle: LogThrottleService,

    #[inject(component)]
    job_indicator_value_service: JobIndicatorValueService,
}

#[async_trait]
impl BaseApp for DefaultJobCallBack {
    fn name(&self) -> &'static str {
        "DefaultJobCallBack"
    }
}

#[async_trait]
impl JobCallBack for DefaultJobCallBack {
    async fn do_call_back(&self, ctx: &CollectContext, result: anyhow::Result<Vec<JobIndicatorValue>>) {
        let mut values = match result {
            Ok(values) => values,
            Err(error) => {
                let text = error.to_string();
                if self.log_throttle.allow(format!("collect.callback.error.{}", ctx.job_id()), &text) {
                    debug!("[collect] job_id={} error={}", ctx.job_id(), text);
                }
                vec![ctx.new_value(None, false, Some(text))]
            }
        };
        let size = values.len();
        if let Err(error) = self.job_indicator_value_service.push_all(&mut values).await {
            let text = error.to_string();
            if self.log_throttle.allow(format!("collect.callback.push.{}", ctx.job_id()), &text) {
                warn!("[collect] job_id={} push value failed: {}", ctx.job_id(), text);
            }
        }
        if let Err(error) = self.alert_evaluator.evaluate_all(ctx, &values).await {
            let text = error.to_string();
            if self.log_throttle.allow(format!("collect.callback.alert.{}", ctx.job_id()), &text) {
                warn!("[collect] job_id={} evaluate alert failed: {}", ctx.job_id(), text);
            }
        }
        if let Err(error) = self.job_indicator_value_service.notify_metric(&ctx.job_device_id(), &ctx.job_indicator_id()).await {
            let text = error.to_string();
            if self.log_throttle.allow(format!("collect.callback.notify.{}", ctx.job_id()), &text) {
                warn!("[collect] job_id={} notify metric failed: {}", ctx.job_id(), text);
            }
        }
        debug!("[collect] job_id={} values={}", ctx.job_id(), size);
    }
}
