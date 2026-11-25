use anyhow::Result;
use std::sync::Arc;
use sea_orm::EntityTrait;
use summer::tracing::warn;
use tokio::sync::broadcast;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::model::ai_provide;
use sea_orm::PrimaryKeyTrait;
use termini_macros::base_app;
use crate::dao::dao_base::BaseDao;
use crate::module::common::cloud_provide;
use crate::model::base_target::BaseTarget;
use crate::model::dto::shared_ptr::Shared;
use crate::module::ai::dto::AiProviderType;
use crate::dao::dao_ai_provide::AiProvideDao;
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultCode;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
use crate::module::ai::code_index::embed_client;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_config::ConfigService;
use crate::service::service_base_api::BaseApiService;
use crate::module::ai::agent::agent_build::probe_agent;
use crate::module::ai::provide::provide_factory::ProvideFactory;
use crate::model::resp::resp_ai_provider_info::RespAiProviderInfo;
use crate::module::ai::provide::provide_template::{build_template_provider_info, resolve_max_token};

#[base_app(sort = 200)]
#[derive(Clone, Service)]
pub struct AiProvideService {
    #[inject(component)]
    ai_provide_dao: AiProvideDao,

    #[inject(component)]
    provide_factory: ProvideFactory,

    #[inject(component)]
    config_service: ConfigService,

    #[inject(func = build_template_provider_info())]
    template_provider_info: Arc<Vec<RespAiProviderInfo>>,
}

#[async_trait]
impl BaseApp for AiProvideService {
    fn name(&self) -> &'static str {
        "AiProvideService"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        match self.fetch_cloud_models().await {
            Ok(list) => *cloud_provide::cloud_list().write() = list,
            Err(err) => summer::tracing::warn!("[ai] fetch cloud models failed: {err:#}"),
        }
        let svc = self.clone();
        tokio::spawn(async move {
            let mut rx = ConfigService::subscribe_config_changed();
            loop {
                match rx.recv().await {
                    Ok(()) | Err(broadcast::error::RecvError::Lagged(_)) => {
                        if let Err(err) = svc.refresh_cloud().await {
                            summer::tracing::warn!("[ai] 配置版本变化后刷新云端模型失败: {err:#}");
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        });
        Ok(())
    }
}

#[async_trait]
impl BaseService for AiProvideService {
    type Entity = ai_provide::Entity;

    fn get_dao(&self) -> Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.ai_provide_dao.clone()))
    }

    async fn find_all(&self) -> Result<Vec<Shared<<Self::Entity as EntityTrait>::Model>>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + Send + Sync,
    {
        let mut local = self.ai_provide_dao.find_all().await?;
        self.sort_provide_list(&mut local);
        let mut list: Vec<Shared<ai_provide::Model>> = cloud_provide::cloud_list().read().iter().cloned().collect();
        list.extend(local);
        Ok(list)
    }

    async fn find_by_id(&self, id: String) -> Result<Option<Shared<<Self::Entity as EntityTrait>::Model>>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + Send + Sync,
        <<Self::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType: From<String>,
    {
        if let Some(found) = cloud_provide::cloud_list().read().iter().find(|item| item.read().id == id).cloned() {
            return Ok(Some(found));
        }
        self.ai_provide_dao.find_by_id(id).await
    }

    async fn insert(&self, entity: <Self::Entity as EntityTrait>::Model) -> Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + sea_orm::IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: sea_orm::ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        self.verify_model_available(&entity).await?;
        self.ai_provide_dao.insert(entity).await
    }

    async fn update(&self, entity: <Self::Entity as EntityTrait>::Model) -> Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + sea_orm::IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: sea_orm::ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        self.verify_model_available(&entity).await?;
        self.ai_provide_dao.update(entity).await
    }
}

impl BaseApiService for AiProvideService {}

impl AiProvideService {
    pub async fn refresh_cloud(&self) -> Result<Vec<Shared<ai_provide::Model>>> {
        let list = self.fetch_cloud_models().await?;
        *cloud_provide::cloud_list().write() = list;
        self.find_all().await
    }

    async fn fetch_cloud_models(&self) -> Result<Vec<Shared<ai_provide::Model>>> {
        let key = Self::api_token();
        let data: Vec<ai_provide::Model> = Self::api_data(Self::api_get("/api/aiProvideApi/selectAll")).await?;
        let url = format!("{}/api/aiProxyApi/v1", Self::api_base());
        Ok(data
            .into_iter()
            .map(|mut item| {
                item.url = url.clone();
                item.key = key.clone();
                Shared::new(item)
            })
            .collect())
    }

    pub fn list_template_provider_info(&self) -> &[RespAiProviderInfo] {
        &self.template_provider_info
    }

    pub fn get_template_provider_info(&self, provide_id: i16) -> Option<&RespAiProviderInfo> {
        self.template_provider_info.iter().find(|item| item.provide_id == provide_id)
    }

    pub fn resolve_token_limit(&self, provide: &ai_provide::Model) -> Result<usize> {
        if let Some(value) = provide.max_token.filter(|value| *value > 0) {
            return Ok(value as usize);
        }
        let provider_code = provide.provider.ok_or_else(|| anyhow::Error::from(ResultCode::ModelMaxTokenMissing))?;
        let provider_type = AiProviderType::from_i16(provider_code).ok_or_else(|| anyhow::Error::from(ResultCode::ModelMaxTokenMissing))?;
        let value = resolve_max_token(provider_type, &provide.model).ok_or_else(|| anyhow::Error::from(ResultCode::ModelMaxTokenMissing))?;
        Ok(value as usize)
    }

    pub async fn resolve_assist(&self) -> Option<ai_provide::Model> {
        let config = self.config_service.select_config().await?;
        let provide_id = config.resolve_assist_provide_id()?;
        match self.find_by_id(provide_id.clone()).await {
            Ok(Some(shared)) => Some(shared.read().clone()),
            Ok(None) => {
                warn!("[ai] 辅助模型未在可用模型里找到, 回退对话模型 provide_id={}", provide_id);
                None
            }
            Err(error) => {
                warn!("[ai] 解析辅助模型失败, 回退对话模型 provide_id={} error={:#}", provide_id, error);
                None
            }
        }
    }

    async fn verify_model_available(&self, entity: &ai_provide::Model) -> Result<()> {
        if entity.kind == ai_provide::KIND_IMAGE_GEN || entity.kind == ai_provide::KIND_VIDEO {
            return Ok(());
        }
        if entity.kind == ai_provide::KIND_EMBEDDING {
            #[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
            {
                if entity.provider == Some(AiProviderType::Anthropic.as_i16()) {
                    return Err(ResultCode::AiModelUnavailable.into());
                }
                if let Err(_error) = embed_client::probe_embedding(entity).await {
                    return Err(ResultCode::AiModelUnavailable.into());
                }
                return Ok(());
            }
            #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
            {
                return Err(ResultCode::AiModelUnavailable.into());
            }
        }
        if let Err(_error) = probe_agent(&self.provide_factory, entity).await {
            return Err(ResultCode::AiModelUnavailable.into());
        }
        Ok(())
    }

    fn resolve_provider_name(&self, provide: &ai_provide::Model) -> String {
        let Some(provider_id) = provide.provider else {
            return "Custom".to_string();
        };
        let Some(info) = self.get_template_provider_info(provider_id) else {
            return "Custom".to_string();
        };
        info.name.clone()
    }

    fn sort_provide_list(&self, list: &mut Vec<Shared<ai_provide::Model>>) {
        list.sort_by(|left, right| {
            let left_model = left.read();
            let right_model = right.read();
            let by_provider = self.resolve_provider_name(&left_model).cmp(&self.resolve_provider_name(&right_model));
            if by_provider != std::cmp::Ordering::Equal {
                return by_provider;
            }
            left_model.model.cmp(&right_model.model)
        });
    }
}
