use std::sync::Arc;
use dashmap::DashMap;
use std::str::FromStr;
use std::time::Duration;
use summer::tracing::debug;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use serde::{Deserialize, Serialize};
use crate::util::time_util::now_millis;
use crate::collect::dto::CollectContext;
use crate::collect::collector::Collector;
use crate::collect::collector::AgentCollector;
use crate::service::service_base::BaseService;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_device::DeviceService;
use crate::model::dto::job::indicator_arg::IndicatorArg;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::service::service_config_snmp::ConfigSnmpService;
use crate::service::service_job_indicator::JobIndicatorService;
use crate::collect::collector::r#impl::parser::parser_transform;
use protocol_snmp::{Oid, SnmpClient, SnmpClientConfig, SnmpVersion, VarBind};
use crate::model::dto::job::indicator_arg_template::{IndicatorArgTemplate, IndicatorArgType};
use crate::model::job_indicator::{JobIndicator, JobIndicatorAlertRuleOp, JobIndicatorDisplayUnit, JobIndicatorParserType, JobIndicatorProtocol};

pub const SNMP_ARG_ID: &str = "snmp-spec";
pub const SNMP_ARG_LABEL_I18N: &str = "job.indicator.arg.snmp.spec";

pub const SNMP_OP_GET: &str = "get";
pub const SNMP_OP_WALK: &str = "walk";
pub const SNMP_WALK_MAX_ROWS: usize = 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnmpIndicatorSpec {
    pub op: String,
    pub oids: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reduce: Option<String>,

    #[serde(default)]
    pub rate: bool,
}

#[derive(Clone, Copy)]
enum SnmpReduce {
    First,
    Sum,
    Avg,
    Max,
    Min,
    Count,
}

impl SnmpReduce {
    fn from_name(name: &str) -> anyhow::Result<Self> {
        match name {
            "first" => Ok(SnmpReduce::First),
            "sum" => Ok(SnmpReduce::Sum),
            "avg" => Ok(SnmpReduce::Avg),
            "max" => Ok(SnmpReduce::Max),
            "min" => Ok(SnmpReduce::Min),
            "count" => Ok(SnmpReduce::Count),
            other => anyhow::bail!("unknown reduce method reduce=\"{}\", supported: first/sum/avg/max/min/count", other),
        }
    }

    fn apply(&self, values: &[f64]) -> f64 {
        match self {
            SnmpReduce::First => values[0],
            SnmpReduce::Sum => values.iter().sum(),
            SnmpReduce::Avg => values.iter().sum::<f64>() / values.len() as f64,
            SnmpReduce::Max => values.iter().cloned().fold(f64::MIN, f64::max),
            SnmpReduce::Min => values.iter().cloned().fold(f64::MAX, f64::min),
            SnmpReduce::Count => values.len() as f64,
        }
    }
}

fn create_rate_cache() -> Arc<DashMap<String, (f64, i64)>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 254)]
#[derive(Clone, Service)]
pub struct CollectorSnmp {
    #[inject(component)]
    device_service: DeviceService,

    #[inject(component)]
    agent_collector: AgentCollector,

    #[inject(component)]
    config_snmp_service: ConfigSnmpService,

    #[inject(component)]
    job_indicator_service: JobIndicatorService,

    #[inject(func = create_rate_cache())]
    rate_cache: Arc<DashMap<String, (f64, i64)>>,
}

#[async_trait]
impl BaseApp for CollectorSnmp {
    fn name(&self) -> &'static str {
        "CollectorSnmp"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.agent_collector.register(Arc::new(self.clone()));
        self.seed_default_indicators().await?;
        Ok(())
    }
}

#[async_trait]
impl Collector for CollectorSnmp {
    fn get_name(&self) -> &'static str {
        "CollectorSnmp"
    }

    fn get_protocol(&self) -> i16 {
        JobIndicatorProtocol::Snmp as i16
    }

    fn get_arg_template(&self) -> IndicatorArgTemplate {
        IndicatorArgTemplate { id: SNMP_ARG_ID.to_string(), arg_type: IndicatorArgType::Snmp as i16, label_i18n: SNMP_ARG_LABEL_I18N.to_string(), protocol: self.get_protocol() }
    }

    async fn collection(&self, ctx: &CollectContext) -> anyhow::Result<Vec<JobIndicatorValue>> {
        let Some(raw_spec) = ctx.script() else {
            anyhow::bail!("this indicator has no SNMP collection parameters configured");
        };
        let spec: SnmpIndicatorSpec = serde_json::from_str(&raw_spec).map_err(|error| anyhow::anyhow!("SNMP collection parameters are not valid JSON: {}", error))?;
        let client = self.build_client(&ctx.device_id()).await?;
        let value = match self.sample(&client, &spec, ctx).await {
            Ok(Some(text)) => match parser_transform::transform(&ctx.transform_expr(), &text) {
                Ok(value) => ctx.new_value(Some(value), true, None),
                Err(error) => ctx.new_value(None, false, Some(format!("value transform script failed, fix it and retry: {}", error))),
            },
            Ok(None) => {
                debug!("[snmp] job_id={} 首个计数器样本已缓存，等待下一轮计算速率", ctx.job_id());
                return Ok(Vec::new());
            }
            Err(error) => ctx.new_value(None, false, Some(format!("SNMP collection failed: {}", error))),
        };
        debug!("[snmp] job_id={} success={} value={:?}", ctx.job_id(), value.success, value.value);
        Ok(vec![value])
    }
}

const SNMP_DEFAULT_TEMPLATES_JSON: &str = include_str!("../../../../resource/snmp_default_templates.json");

#[derive(Debug, Deserialize)]
struct SnmpDefaultTemplate {
    id: String,
    name: String,
    sort: i16,
    unit: String,
    spec: String,
    transform_expr: String,
    display_unit: String,
}

impl SnmpDefaultTemplate {
    fn resolve_display_unit(&self) -> anyhow::Result<i16> {
        if self.display_unit == "none" {
            return Ok(JobIndicatorDisplayUnit::None as i16);
        }
        let unit =
            JobIndicatorDisplayUnit::from_name(&self.display_unit).ok_or_else(|| anyhow::anyhow!("built-in SNMP template {} has an unrecognized display_unit: {}", self.id, self.display_unit))?;
        Ok(unit as i16)
    }
}

impl CollectorSnmp {
    async fn seed_default_indicators(&self) -> anyhow::Result<()> {
        let templates: Vec<SnmpDefaultTemplate> = serde_json::from_str(SNMP_DEFAULT_TEMPLATES_JSON)?;
        for template in &templates {
            serde_json::from_str::<SnmpIndicatorSpec>(&template.spec).map_err(|error| anyhow::anyhow!("built-in SNMP template {} has an invalid spec: {}", template.id, error))?;
            if self.job_indicator_service.find_by_id(template.id.clone()).await?.is_some() {
                continue;
            }
            let arg = IndicatorArg { unit: template.unit.clone(), value: template.spec.clone(), template: self.get_arg_template() };
            let indicator = JobIndicator {
                id: template.id.clone(),
                name: template.name.clone(),
                sort: template.sort,
                enabled: true,
                protocol: JobIndicatorProtocol::Snmp as i16,
                arg: serde_json::to_value(&arg)?,
                is_default: true,
                parser_type: JobIndicatorParserType::ExitCode as i16,
                parse_expr: String::new(),
                display_unit: template.resolve_display_unit()?,
                alert_enabled: false,
                alert_ai_on: 0,
                alert_rule_op: JobIndicatorAlertRuleOp::Gt as i16,
                alert_threshold: String::new(),
                transform_expr: template.transform_expr.clone(),
                alert_sensitivity: 1,
                alert_ai_repair_message: None,
                range_min: None,
                range_max: None,
            };
            self.job_indicator_service.insert(indicator).await?;
            debug!("[snmp] 内置指标模板已写入: {}", template.id);
        }
        Ok(())
    }

    pub async fn build_client(&self, device_id: &str) -> anyhow::Result<SnmpClient> {
        let Some(device_ptr) = self.device_service.find_by_id(device_id.to_string()).await? else {
            anyhow::bail!("device not found, it may have been deleted");
        };
        let host = device_ptr.read().host.clone();
        if host.trim().is_empty() {
            anyhow::bail!("device has no IP or hostname configured");
        }
        let Some(config_ptr) = self.config_snmp_service.get_by_device_id(device_id.to_string()) else {
            anyhow::bail!("device has no SNMP config; enable SNMP on the device edit page");
        };
        let config = config_ptr.read().clone();
        if !config.enabled {
            anyhow::bail!("SNMP is not enabled on this device; enable it and set the community on the device edit page");
        }
        if config.community.trim().is_empty() {
            anyhow::bail!("SNMP community is empty");
        }
        let version = SnmpVersion::from_number(config.version as i32).map_err(|error| anyhow::anyhow!("{}", error))?;
        let client_config = SnmpClientConfig {
            host,
            port: config.port as u16,
            version,
            community: config.community.clone(),
            timeout: Duration::from_secs(config.timeout_seconds.max(1) as u64),
            retries: config.retries.max(0) as u32,
        };
        let client = SnmpClient::connect(client_config).await.map_err(|error| anyhow::anyhow!("SNMP connection initialization failed: {}", error))?;
        Ok(client)
    }

    async fn sample(&self, client: &SnmpClient, spec: &SnmpIndicatorSpec, ctx: &CollectContext) -> anyhow::Result<Option<String>> {
        if spec.oids.is_empty() {
            anyhow::bail!("oids cannot be empty");
        }
        let oids = spec.oids.iter().map(|raw| Oid::from_str(raw).map_err(|error| anyhow::anyhow!("{}", error))).collect::<anyhow::Result<Vec<Oid>>>()?;

        let varbinds = match spec.op.as_str() {
            SNMP_OP_GET => client.get(&oids).await.map_err(|error| anyhow::anyhow!("{}", error))?,
            SNMP_OP_WALK => {
                if oids.len() != 1 {
                    anyhow::bail!("walk accepts exactly one root OID, got {}", oids.len());
                }
                let rows = client.walk(&oids[0], SNMP_WALK_MAX_ROWS).await.map_err(|error| anyhow::anyhow!("{}", error))?;
                if rows.is_empty() {
                    anyhow::bail!("walk {} returned no rows, check that the OID exists", oids[0]);
                }
                rows
            }
            other => anyhow::bail!("unknown operation op=\"{}\", supported: get/walk", other),
        };

        if varbinds.is_empty() {
            anyhow::bail!("SNMP response carried no values");
        }
        for varbind in &varbinds {
            if varbind.value.is_exception() {
                anyhow::bail!("OID {} returned {}, the object does not exist on the target device", varbind.oid, varbind.value.type_name());
            }
        }

        if spec.op == SNMP_OP_GET && varbinds.len() == 1 && spec.reduce.is_none() && !spec.rate {
            return Ok(Some(varbinds[0].value.to_string()));
        }

        let numbers = Self::to_numbers(&varbinds)?;
        let reduce = match &spec.reduce {
            Some(name) => SnmpReduce::from_name(name)?,
            None => {
                if numbers.len() > 1 {
                    anyhow::bail!("{} values returned, a reduce method must be specified (sum/avg/max/min/count/first)", numbers.len());
                }
                SnmpReduce::First
            }
        };
        let aggregated = reduce.apply(&numbers);

        if !spec.rate {
            return Ok(Some(Self::format_number(aggregated)));
        }
        Ok(self.compute_rate(ctx, aggregated).map(|rate| Self::format_number(rate)))
    }

    /// Counter 速率：用上一轮样本做差除以间隔秒数。首轮只建基线不出值；
    /// 计数器回绕/重置（当前值小于上一轮）时重建基线，不编造数据。
    fn compute_rate(&self, ctx: &CollectContext, current: f64) -> Option<f64> {
        let key = ctx.job_id();
        let now = now_millis();
        let previous = self.rate_cache.insert(key, (current, now));
        let Some((prev_value, prev_at)) = previous else {
            return None;
        };
        let elapsed_secs = (now - prev_at) as f64 / 1000.0;
        if elapsed_secs <= 0.0 {
            return None;
        }
        if current < prev_value {
            debug!("[snmp] job_id={} 计数器回绕或重置（{} -> {}），本轮重建基线", ctx.job_id(), prev_value, current);
            return None;
        }
        Some((current - prev_value) / elapsed_secs)
    }

    fn to_numbers(varbinds: &[VarBind]) -> anyhow::Result<Vec<f64>> {
        let mut numbers = Vec::with_capacity(varbinds.len());
        for varbind in varbinds {
            let Some(number) = varbind.value.as_f64() else {
                anyhow::bail!("OID {} has value type {}, not a number, cannot aggregate", varbind.oid, varbind.value.type_name());
            };
            numbers.push(number);
        }
        Ok(numbers)
    }

    fn format_number(value: f64) -> String {
        if (value - value.round()).abs() < 1e-9 {
            return format!("{}", value.round() as i64);
        }
        let text = format!("{:.4}", value);
        text.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}
