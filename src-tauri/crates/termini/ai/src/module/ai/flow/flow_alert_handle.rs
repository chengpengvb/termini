use anyhow::Result;
use std::sync::Arc;
use parking_lot::Mutex;
use summer::plugin::Service;
use async_trait::async_trait;
use rig::completion::Message;
use termini_macros::base_app;
use crate::dao::dao_base::BaseDao;
use summer::tracing::{info, warn};
use crate::dao::dao_device::DeviceDao;
use crate::model::ai_chat::AiChatType;
use crate::model::device::DeviceOsType;
use crate::util::time_util::now_millis;
use tokio_util::sync::CancellationToken;
use crate::dao::dao_job_device::JobDeviceDao;
use crate::module::ai::shared::AiTaskSession;
use crate::model::job_indicator::JobIndicator;
use crate::service::service_base::BaseService;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_config::ConfigService;
use crate::dao::dao_job_indicator::JobIndicatorDao;
use crate::service::service_ai_chat::AiChatService;
use crate::module::ai::agent::agent_alert::AgentAlert;
use crate::model::job_indicator_alert::JobIndicatorAlert;
use crate::model::job_indicator_value::JobIndicatorValue;
use crate::module::ai::stream::stream_runner::StreamErrorEmitted;
use crate::service::service_job_indicator_alert::JobIndicatorAlertService;
use crate::service::service_job_indicator_value::JobIndicatorValueService;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use crate::model::req::req_chat_context::{ChatOrigin, ConfirmMode, ReqChatContext};
use crate::model::job_indicator::{JobIndicatorAlertAiOn, JobIndicatorAlertRuleOp, JobIndicatorParserType, JobIndicatorProtocol};
use crate::module::ai::agent::prompts::{output_language_fixed_text, render, LEAD_ALERT_GRANT_ANALYZE, LEAD_ALERT_GRANT_REPAIR, LEAD_ALERT_USER};

const RECENT_SAMPLE_LIMIT: usize = 12;

struct DiagnoseQueue {
    tx: UnboundedSender<String>,
    rx: Mutex<Option<UnboundedReceiver<String>>>,
}

fn create_diagnose_queue() -> Arc<DiagnoseQueue> {
    let (tx, rx) = unbounded_channel();
    Arc::new(DiagnoseQueue { tx, rx: Mutex::new(Some(rx)) })
}

#[base_app(sort = 770)]
#[derive(Clone, Service)]
pub struct FlowAlertHandle {
    #[inject(component)]
    agent_alert: AgentAlert,

    #[inject(component)]
    device_dao: DeviceDao,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    job_device_dao: JobDeviceDao,

    #[inject(component)]
    ai_task_session: AiTaskSession,

    #[inject(component)]
    ai_chat_service: AiChatService,

    #[inject(component)]
    job_indicator_dao: JobIndicatorDao,

    #[inject(func = create_diagnose_queue())]
    diagnose_queue: Arc<DiagnoseQueue>,

    #[inject(component)]
    job_indicator_alert_service: JobIndicatorAlertService,

    #[inject(component)]
    job_indicator_value_service: JobIndicatorValueService,
}

#[async_trait]
impl BaseApp for FlowAlertHandle {
    fn name(&self) -> &'static str {
        "FlowAlertHandle"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        let hook_flow = self.clone();
        crate::module::common::alert_hook::register_alert_enqueue(move |alert_id| hook_flow.enqueue(alert_id));
        let Some(mut rx) = self.diagnose_queue.rx.lock().take() else {
            return Ok(());
        };
        let flow = self.clone();
        tokio::spawn(async move {
            while let Some(alert_id) = rx.recv().await {
                if let Err(error) = flow.diagnose(alert_id.clone()).await {
                    warn!("[ai] 诊断失败 alert_id={} error={:#}", alert_id, error);
                }
            }
        });
        Ok(())
    }
}

impl FlowAlertHandle {
    pub fn enqueue(&self, alert_id: String) {
        if let Err(error) = self.diagnose_queue.tx.send(alert_id) {
            warn!("[ai] 诊断队列已关闭，丢弃 alert_id={}", error.0);
        }
    }

    async fn diagnose(&self, alert_id: String) -> Result<()> {
        let Some(shared) = self.job_indicator_alert_service.find_by_id(alert_id.clone()).await? else {
            return Ok(());
        };
        let alert = shared.read().clone();
        let Some(indicator) = self.job_indicator_dao.find_by_id(alert.job_indicator_id.clone()).await? else {
            return Ok(());
        };
        let indicator = indicator.read().clone();
        let Some(grant) = JobIndicatorAlertAiOn::from_i16(indicator.alert_ai_on) else {
            return Ok(());
        };
        let Some(ai_provide_id) = self.resolve_ai_provide_id().await else {
            warn!("[ai] 未配置告警诊断模型，跳过 alert_id={}", alert_id);
            return Ok(());
        };

        let title = format!("告警诊断 · {}", indicator.name);
        let chat = self.ai_chat_service.start_task(AiChatType::Alert, title).await?;
        self.save_chat_id(&alert, &chat.id).await?;
        let prompt = self.build_prompt(&alert, &indicator).await?;
        info!("[ai] 开始诊断 alert_id={} chat_id={} 授权={:?}", alert_id, chat.id, grant);
        let context = ReqChatContext {
            ai_provide_id,
            images: Vec::new(),
            thinking_level: None,
            message: prompt.clone(),
            origin: ChatOrigin::Alert,
            ai_chat_id: chat.id.clone(),
            confirm_mode: Self::confirm_mode(grant),
        };
        let cancel_token = CancellationToken::new();
        let run = self.ai_task_session.open(context, cancel_token.clone()).await?;
        let mut addon_messages = vec![Message::user(Self::grant_text(grant)), Message::user(output_language_fixed_text(self.config_service.select_system_language().await.label()))];
        if let Some(repair_message) = Self::repair_message(grant, &indicator) {
            addon_messages.push(Message::user(repair_message));
        }
        let result = self.ai_task_session.run::<AgentAlert, String>(&run, &self.agent_alert, prompt, addon_messages).await;
        let error = result.as_ref().err().filter(|error| !StreamErrorEmitted::displayed(error)).map(|error| format!("{:#}", error));
        self.ai_task_session.close(run, error).await;
        if cancel_token.is_cancelled() {
            info!("[ai] 诊断已中断，不写结论 alert_id={}", alert_id);
            return Ok(());
        }

        let summary = result?;
        self.save_summary(&alert_id, summary).await?;
        Ok(())
    }

    fn grant_text(grant: JobIndicatorAlertAiOn) -> &'static str {
        match grant {
            JobIndicatorAlertAiOn::RepairConfirm => LEAD_ALERT_GRANT_REPAIR,
            _ => LEAD_ALERT_GRANT_ANALYZE,
        }
    }

    fn confirm_mode(grant: JobIndicatorAlertAiOn) -> ConfirmMode {
        match grant {
            JobIndicatorAlertAiOn::RepairConfirm => ConfirmMode::Auto,
            _ => ConfirmMode::Silent,
        }
    }

    fn repair_message(grant: JobIndicatorAlertAiOn, indicator: &JobIndicator) -> Option<String> {
        if grant != JobIndicatorAlertAiOn::RepairConfirm {
            return None;
        }
        let message = indicator.alert_ai_repair_message.as_deref()?.trim();
        if message.is_empty() {
            return None;
        }
        Some(format!("[用户修复说明]\n{}", message))
    }

    async fn resolve_ai_provide_id(&self) -> Option<String> {
        let id = self.config_service.select_config().await?.job_alert_ai_provide_id?;
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(trimmed.to_string())
    }

    async fn save_chat_id(&self, alert: &JobIndicatorAlert, chat_id: &str) -> Result<()> {
        let mut next = alert.clone();
        next.ai_chat_id = Some(chat_id.to_string());
        next.job_indicator = None;
        self.job_indicator_alert_service.update(next).await?;
        Ok(())
    }

    async fn save_summary(&self, alert_id: &str, summary: String) -> Result<()> {
        let summary = summary.trim();
        if summary.is_empty() {
            return Ok(());
        }
        let Some(shared) = self.job_indicator_alert_service.find_by_id(alert_id.to_string()).await? else {
            return Ok(());
        };
        let mut alert = shared.read().clone();
        alert.job_indicator = None;
        alert.ai_summary = Some(summary.to_string());
        self.job_indicator_alert_service.update(alert).await?;
        info!("[ai] 诊断结论已写回 alert_id={}", alert_id);
        Ok(())
    }

    async fn build_prompt(&self, alert: &JobIndicatorAlert, indicator: &JobIndicator) -> Result<String> {
        let (device_id, device_name, device_host, device_os, device_profile) = self.resolve_device(&alert.job_device_id).await?;
        let arg = indicator.get_arg()?.unwrap_or_default();
        let value = alert.get_job_indicator_value()?.unwrap_or_default();
        let samples = self.job_indicator_value_service.list_by_range(&alert.job_device_id, &alert.job_indicator_id, 0);

        Ok(render(
            LEAD_ALERT_USER,
            &[
                ("device_os", device_os),
                ("device_id", device_id.as_str()),
                ("device_name", device_name.as_str()),
                ("device_host", device_host.as_str()),
                ("device_profile", device_profile.as_str()),
                ("indicator_name", indicator.name.as_str()),
                ("indicator_unit", Self::or_dash(&arg.unit)),
                ("alert_at", Self::ago(alert.alert_at).as_str()),
                ("indicator_command", Self::or_dash(&arg.value)),
                ("alert_op", Self::op_label(alert.alert_rule_op)),
                ("alert_value", Self::or_dash(&alert.alert_value)),
                ("alert_threshold", alert.alert_threshold.as_str()),
                ("recent_samples", Self::format_samples(&samples).as_str()),
                ("alert_success", if value.success { "成功" } else { "失败" }),
                ("indicator_parse_expr", Self::or_dash(&indicator.parse_expr)),
                ("indicator_parser", Self::parser_label(indicator.parser_type)),
                ("indicator_protocol", Self::protocol_label(indicator.protocol)),
                ("alert_error", Self::or_dash(value.error.as_deref().unwrap_or_default())),
            ],
        ))
    }

    async fn resolve_device(&self, job_device_id: &str) -> Result<(String, String, String, &'static str, String)> {
        let empty_profile = Self::profile_text(None);
        let Some(job_device) = self.job_device_dao.find_by_id(job_device_id.to_string()).await? else {
            return Ok((String::new(), "未知设备".to_string(), String::new(), DeviceOsType::Unknown.label(), empty_profile));
        };
        let device_id = job_device.read().device_id.clone();
        let Some(device) = self.device_dao.find_by_id(device_id.clone()).await? else {
            return Ok((device_id, "未知设备".to_string(), String::new(), DeviceOsType::Unknown.label(), empty_profile));
        };
        let device = device.read();
        Ok((device_id, device.name.clone(), device.host.clone(), DeviceOsType::from_i16(device.os_type).label(), Self::profile_text(device.profile.as_deref())))
    }

    fn profile_text(profile: Option<&str>) -> String {
        match profile.map(str::trim).filter(|text| !text.is_empty()) {
            Some(text) => text.to_string(),
            None => "（这台设备还没有档案，跑一次指标探索后会自动沉淀）".to_string(),
        }
    }

    fn format_samples(samples: &[JobIndicatorValue]) -> String {
        if samples.is_empty() {
            return "（内存里还没有历史样本）".to_string();
        }
        let start = samples.len().saturating_sub(RECENT_SAMPLE_LIMIT);
        samples[start..]
            .iter()
            .map(|sample| {
                let value = sample.value.as_deref().unwrap_or("无值");
                match sample.error.as_deref() {
                    Some(error) => format!("{}：{}（采集出错：{}）", Self::ago(sample.updated_at), value, error),
                    None => format!("{}：{}", Self::ago(sample.updated_at), value),
                }
            })
            .collect::<Vec<String>>()
            .join("\n")
    }

    fn or_dash(text: &str) -> &str {
        if text.trim().is_empty() {
            return "无";
        }
        text
    }

    fn ago(timestamp: i64) -> String {
        let seconds = now_millis().saturating_sub(timestamp).max(0) / 1000;
        if seconds < 60 {
            return format!("{} 秒前", seconds);
        }
        if seconds < 3600 {
            return format!("{} 分钟前", seconds / 60);
        }
        if seconds < 86400 {
            return format!("{} 小时前", seconds / 3600);
        }
        format!("{} 天前", seconds / 86400)
    }

    fn protocol_label(protocol: i16) -> &'static str {
        match protocol {
            value if value == JobIndicatorProtocol::SSH as i16 => "SSH 执行命令",
            value if value == JobIndicatorProtocol::Telnet as i16 => "Telnet 执行命令",
            value if value == JobIndicatorProtocol::Icmp as i16 => "ICMP 探测",
            _ => "未知方式",
        }
    }

    fn parser_label(parser_type: i16) -> &'static str {
        match JobIndicatorParserType::from_i16(parser_type) {
            Some(JobIndicatorParserType::ExitCode) => "取退出码",
            Some(JobIndicatorParserType::Regex) => "正则提取",
            Some(JobIndicatorParserType::JsonPath) => "JSONPath 提取",
            None => "未知解析方式",
        }
    }

    fn op_label(alert_rule_op: i16) -> &'static str {
        match JobIndicatorAlertRuleOp::from_i16(alert_rule_op) {
            Some(JobIndicatorAlertRuleOp::Gt) => "大于",
            Some(JobIndicatorAlertRuleOp::Gte) => "大于等于",
            Some(JobIndicatorAlertRuleOp::Lt) => "小于",
            Some(JobIndicatorAlertRuleOp::Lte) => "小于等于",
            Some(JobIndicatorAlertRuleOp::Eq) => "等于",
            None => "未知条件",
        }
    }
}
