use serde_json::Value;
use summer::plugin::Service;
use anyhow::{bail, Context, Result};
use crate::model::config::{WEBHOOK_KIND_DINGTALK, WEBHOOK_KIND_FEISHU, WEBHOOK_KIND_TELEGRAM, WEBHOOK_KIND_WECOM};

#[derive(Clone, Service)]
pub struct NotifyWebhookService {}

impl NotifyWebhookService {
    pub async fn send(&self, kind: i16, url: &str, extra: Option<&str>, text: &str) -> Result<()> {
        let body = Self::build_body(kind, extra, text)?;
        let res = reqwest::Client::new().post(url).json(&body).send().await.context("request webhook")?;
        let status = res.status();
        let payload = res.text().await.context("read webhook response")?;
        if !status.is_success() {
            bail!("webhook http {}: {}", status, payload);
        }
        Self::ensure_ok(kind, &payload)
    }

    fn build_body(kind: i16, extra: Option<&str>, text: &str) -> Result<Value> {
        match kind {
            WEBHOOK_KIND_DINGTALK | WEBHOOK_KIND_WECOM => Ok(serde_json::json!({"msgtype": "text", "text": {"content": text}})),
            WEBHOOK_KIND_FEISHU => Ok(serde_json::json!({"msg_type": "text", "content": {"text": text}})),
            WEBHOOK_KIND_TELEGRAM => {
                let Some(chat_id) = extra.map(str::trim).filter(|item| !item.is_empty()) else {
                    bail!("telegram chat_id is empty");
                };
                Ok(serde_json::json!({"chat_id": chat_id, "text": text}))
            }
            _ => bail!("unknown webhook kind: {kind}"),
        }
    }

    fn ensure_ok(kind: i16, payload: &str) -> Result<()> {
        let value: Value = serde_json::from_str(payload).context("parse webhook response")?;
        let ok = match kind {
            WEBHOOK_KIND_DINGTALK | WEBHOOK_KIND_WECOM => value.get("errcode").and_then(Value::as_i64) == Some(0),
            WEBHOOK_KIND_FEISHU => value.get("code").and_then(Value::as_i64) == Some(0) || value.get("StatusCode").and_then(Value::as_i64) == Some(0),
            WEBHOOK_KIND_TELEGRAM => value.get("ok").and_then(Value::as_bool) == Some(true),
            _ => false,
        };
        if !ok {
            bail!("{payload}");
        }
        Ok(())
    }
}
