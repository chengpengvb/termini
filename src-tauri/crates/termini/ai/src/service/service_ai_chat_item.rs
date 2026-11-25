use anyhow::Result;
use std::sync::Arc;
use dashmap::DashMap;
use sea_orm::EntityTrait;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use std::collections::HashSet;
use crate::model::ai_chat_item;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use summer::tracing::{info, warn};
use crate::model::base_target::BaseTarget;
use crate::model::dto::shared_ptr::Shared;
use crate::model::ai_chat_item::AiChatItem;
use crate::service::service_base::BaseService;
use crate::dao::dao_ai_chat_item::AiChatItemDao;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::resp::resp_agent::RespAgentQuestion;
use crate::model::resp::event::resp_event_agent::RespEventAgent;

const SETTLED_RESULT: &str = "exit_code=interrupted";

fn create_chat_id_2_item_cache_map() -> Arc<DashMap<String, Vec<AiChatItem>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 202)]
#[derive(Clone, Service)]
pub struct AiChatItemService {
    #[inject(component)]
    ai_chat_item_dao: AiChatItemDao,

    #[inject(func = create_chat_id_2_item_cache_map())]
    chat_id_2_item_cache: Arc<DashMap<String, Vec<AiChatItem>>>,
}

#[async_trait]
impl BaseApp for AiChatItemService {
    fn name(&self) -> &'static str {
        "AiChatItemService"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        match self.settle_interrupted().await {
            Ok((0, 0)) => {}
            Ok((calls, runs)) => info!("[ai] 收尾上次中断的工具调用 {} 条, 对话轮次 {} 轮", calls, runs),
            Err(error) => warn!("[ai] 收尾上次中断的记录失败: {error:#}"),
        }
        Ok(())
    }
}

#[async_trait]
impl BaseService for AiChatItemService {
    type Entity = ai_chat_item::Entity;

    fn get_dao(&self) -> Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.ai_chat_item_dao.clone()))
    }

    async fn find_all(&self) -> Result<Vec<Shared<<Self::Entity as EntityTrait>::Model>>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + Send + Sync,
    {
        let mut list = self.ai_chat_item_dao.find_all().await?;
        list.sort_by(|a, b| a.read().timestamp.cmp(&b.read().timestamp));
        Ok(list)
    }
}

impl AiChatItemService {
    pub fn push_item_2_cache(&self, chat_id: String, item: AiChatItem) {
        let mut cache = self.chat_id_2_item_cache.entry(chat_id.clone()).or_insert(Vec::new());
        cache.push(item);
    }

    pub fn get_item_cache(&self, chat_id: String) -> Option<Vec<AiChatItem>> {
        self.chat_id_2_item_cache.get(&chat_id).map(|cache| cache.clone())
    }

    pub fn remove_item_cache(&self, chat_id: String) {
        self.chat_id_2_item_cache.remove(&chat_id);
    }

    pub fn remove_ids_from_cache(&self, chat_id: String, ids: &HashSet<String>) {
        let Some(mut entry) = self.chat_id_2_item_cache.get_mut(&chat_id) else {
            return;
        };
        entry.retain(|item| !ids.contains(&item.id));
        let empty = entry.is_empty();
        drop(entry);
        if empty {
            self.chat_id_2_item_cache.remove(&chat_id);
        }
    }

    pub async fn delete_by_chat_id(&self, chat_id: String) -> Result<u64> {
        self.remove_item_cache(chat_id.clone());
        self.ai_chat_item_dao.delete_by_chat_id(chat_id).await
    }

    pub async fn delete_by_ids(&self, chat_id: String, ids: Vec<String>) -> Result<u64> {
        self.remove_ids_from_cache(chat_id, &ids.iter().cloned().collect());
        self.ai_chat_item_dao.delete_by_ids(ids).await
    }

    pub async fn search_chat_ids(&self, keyword: &str) -> Result<Vec<String>> {
        let keyword = keyword.trim();
        if keyword.is_empty() {
            return Ok(Vec::new());
        }
        self.ai_chat_item_dao.find_chat_ids_by_keyword(keyword, &[RespEventAgent::ON_USER, RespEventAgent::ON_TEXT_FINAL]).await
    }

    pub async fn select_by_chat_id(&self, chat_id: String) -> Result<Vec<Shared<AiChatItem>>> {
        let list = self.ai_chat_item_dao.find_by_chat_id(chat_id.clone()).await?;
        let mut result: Vec<Shared<AiChatItem>> = list.into_iter().map(Shared::new).collect();
        result.sort_by(|a, b| a.read().timestamp.cmp(&b.read().timestamp));
        let mut id_set: HashSet<String> = result.iter().map(|item| item.read().id.clone()).collect();
        if let Some(cache_items) = self.get_item_cache(chat_id) {
            for item in cache_items {
                if id_set.contains(&item.id) {
                    continue;
                }
                id_set.insert(item.id.clone());
                result.push(Shared::new(item));
            }
        }
        Ok(result)
    }

    pub async fn find_question(&self, chat_id: String, call_id: &str) -> Result<Option<(AiChatItem, RespAgentQuestion)>> {
        for shared in self.select_by_chat_id(chat_id).await? {
            let item = shared.read().clone();
            if item.r#type != RespEventAgent::ON_QUESTION {
                continue;
            }
            let question: RespAgentQuestion = serde_json::from_value(item.context.clone())?;
            if question.call_id == call_id {
                return Ok(Some((item, question)));
            }
        }
        Ok(None)
    }

    pub async fn save_question(&self, mut item: AiChatItem, question: &RespAgentQuestion) -> Result<()> {
        item.context = serde_json::to_value(question)?;
        if let Some(mut cache) = self.chat_id_2_item_cache.get_mut(&item.chat_id) {
            for cached in cache.iter_mut().filter(|cached| cached.id == item.id) {
                cached.context = item.context.clone();
            }
        }
        self.ai_chat_item_dao.update(item).await?;
        Ok(())
    }

    pub async fn decline_pending_questions(&self, chat_id: String) -> Result<Vec<(Option<String>, RespAgentQuestion)>> {
        let mut declined = Vec::new();
        for shared in self.select_by_chat_id(chat_id).await? {
            let item = shared.read().clone();
            if item.r#type != RespEventAgent::ON_QUESTION {
                continue;
            }
            let mut question: RespAgentQuestion = serde_json::from_value(item.context.clone())?;
            if question.state.is_some() {
                continue;
            }
            question.state = Some(RespAgentQuestion::STATE_DECLINED.to_string());
            let pid = item.pid.clone();
            self.save_question(item, &question).await?;
            declined.push((pid, question));
        }
        Ok(declined)
    }

    async fn settle_interrupted(&self) -> Result<(usize, usize)> {
        let items = self.ai_chat_item_dao.find_all().await?;
        let mut rows: Vec<AiChatItem> = items.into_iter().map(|item| item.read().clone()).collect();
        rows.sort_by(|left, right| left.timestamp.cmp(&right.timestamp));
        let calls = self.settle_interrupted_calls(&rows).await;
        let runs = self.settle_interrupted_runs(&rows).await;
        Ok((calls, runs))
    }

    async fn settle_interrupted_calls(&self, rows: &[AiChatItem]) -> usize {
        let mut pending: Vec<&AiChatItem> = Vec::new();
        for row in rows {
            let Some(call_id) = Self::context_str(row, "callId") else {
                continue;
            };
            if row.r#type == RespEventAgent::ON_TOOL_CALL {
                pending.push(row);
                continue;
            }
            if row.r#type == RespEventAgent::ON_TOOL_CALL_DONE {
                pending.retain(|item| Self::context_str(item, "callId") != Some(call_id));
            }
        }
        let total = pending.len();
        for call in pending {
            let Some(call_id) = Self::context_str(call, "callId") else {
                continue;
            };
            let tool_id = call.context.get("toolId").cloned().unwrap_or(serde_json::Value::Null);
            let settled = AiChatItem {
                id: uuid(),
                chat_id: call.chat_id.clone(),
                r#type: RespEventAgent::ON_TOOL_CALL_DONE,
                timestamp: call.timestamp + 1,
                context: serde_json::json!({"callId": call_id, "toolId": tool_id, "result": SETTLED_RESULT, "interrupted": true}),
                pid: call.pid.clone(),
            };
            if let Err(error) = self.ai_chat_item_dao.insert(settled).await {
                warn!("[ai] 收尾中断的工具调用失败 chat_id={} call_id={} error={:#}", call.chat_id, call_id, error);
            }
        }
        total
    }

    async fn settle_interrupted_runs(&self, rows: &[AiChatItem]) -> usize {
        let mut open: Vec<(&AiChatItem, i64)> = Vec::new();
        for row in rows {
            let Some(req_id) = Self::context_str(row, "reqId") else {
                continue;
            };
            if row.r#type == RespEventAgent::ON_USER {
                open.push((row, row.timestamp));
                continue;
            }
            if row.r#type == RespEventAgent::ON_DONE || row.r#type == RespEventAgent::ON_CANCELLED {
                open.retain(|(user, _)| Self::context_str(user, "reqId") != Some(req_id));
                continue;
            }
            if let Some(run) = open.iter_mut().find(|(user, _)| Self::context_str(user, "reqId") == Some(req_id)) {
                run.1 = run.1.max(row.timestamp);
            }
        }
        let total = open.len();
        for (user, last_ms) in open {
            let Some(req_id) = Self::context_str(user, "reqId") else {
                continue;
            };
            let mut context = serde_json::json!({"reqId": req_id, "interrupted": true});
            if let Some(parent_req_id) = user.context.get("parentReqId") {
                context["parentReqId"] = parent_req_id.clone();
            }
            let settled = AiChatItem { id: uuid(), chat_id: user.chat_id.clone(), r#type: RespEventAgent::ON_CANCELLED, timestamp: last_ms + 1, context, pid: None };
            if let Err(error) = self.ai_chat_item_dao.insert(settled).await {
                warn!("[ai] 收尾中断的对话轮次失败 chat_id={} req_id={} error={:#}", user.chat_id, req_id, error);
            }
        }
        total
    }

    fn context_str<'a>(item: &'a AiChatItem, key: &str) -> Option<&'a str> {
        item.context.get(key).and_then(|value| value.as_str())
    }
}
