use serde_json::Value;
use crate::model::group;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::dao::dao_base::BaseDao;
use summer::tracing::{error, info};
use crate::dao::dao_group::GroupDao;
use termini_base::submit_sync_entry;
use crate::model::base_target::BaseTarget;
use crate::model::dto::shared_ptr::Shared;
use termini_macros::{base_app, base_service};
use crate::service::service_base::BaseService;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::group::{Group, DEFAULT_GROUP_ID};
use crate::service::service_base_sync::{self, BaseSync};
use crate::model::resp::resp_model::{BizError, ResultCode};
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel};
use crate::service::service_group_2_device::Group2DeviceService;

#[base_app(sort = 200)]
#[base_service(group::Entity, dao = "group_dao", service = false)]
#[derive(Clone, Service)]
pub struct GroupService {
    #[inject(component)]
    group_dao: GroupDao,

    #[inject(component)]
    group_2_device_service: Group2DeviceService,
}

#[async_trait]
impl BaseService for GroupService {
    type Entity = group::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.group_dao.clone()))
    }

    async fn insert(&self, entity: <Self::Entity as EntityTrait>::Model) -> anyhow::Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let list = self.find_all().await?;
        if let Some(found) = list.into_iter().find(|item| item.read().name == entity.name) {
            let existing_id = found.read().id.clone();
            if !self.group_2_device_service.referenced_group_ids().contains(&entity.id) {
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
            if row.id != entity.id && row.name == entity.name {
                if !self.group_2_device_service.referenced_group_ids().contains(&entity.id) {
                    return Err(BizError::duplicate(row.id.clone()).into());
                }
            }
        }
        self.get_dao()?.update(entity).await
    }
}

#[async_trait]
impl BaseApp for GroupService {
    fn name(&self) -> &'static str {
        "GroupService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        self.init_default_groups().await;
        Ok(())
    }
}

impl GroupService {
    async fn init_default_groups(&self) {
        let group_option = match self.group_dao.find_by_id(DEFAULT_GROUP_ID.to_string()).await {
            Ok(group) => group,
            Err(e) => {
                error!("[group] Failed to find default group: {}", e);
                return;
            }
        };
        if group_option.is_some() {
            return;
        }
        let group = Group { id: DEFAULT_GROUP_ID.to_string(), name: "DefaultGroup".to_string() };
        if let Err(e) = self.group_dao.insert(group).await {
            error!("[group] Failed to insert default group: {}", e);
            return;
        }
        info!("[group] 创建默认设备分组[{}].", DEFAULT_GROUP_ID);
    }

    pub async fn create_group(&self, mut group: Group) -> anyhow::Result<Shared<Group>> {
        let name = group.name.trim().to_string();
        if name.is_empty() {
            return Err(ResultCode::GroupNameEmpty.into());
        }
        group.name = name;
        self.ensure_group_name_unique(&group.name, None).await?;
        self.group_dao.insert(group).await
    }

    pub async fn update_group(&self, mut group: Group) -> anyhow::Result<Shared<Group>> {
        let name = group.name.trim().to_string();
        if name.is_empty() {
            return Err(ResultCode::GroupNameEmpty.into());
        }
        group.name = name;
        self.ensure_group_name_unique(&group.name, Some(group.id.as_str())).await?;
        self.group_dao.update(group).await
    }

    pub async fn delete_group(&self, id: String) -> anyhow::Result<()> {
        if id == DEFAULT_GROUP_ID {
            return Err(ResultCode::GroupDefaultProtected.into());
        }
        match self.group_dao.find_by_id(id.clone()).await? {
            Some(_) => {}
            None => return Err(ResultCode::GroupNotFound.into()),
        }
        self.group_2_device_service.reassign_group(&id, DEFAULT_GROUP_ID).await?;
        self.group_dao.delete_by_id(id).await?;
        Ok(())
    }

    async fn ensure_group_name_unique(&self, name: &str, exclude_id: Option<&str>) -> anyhow::Result<()> {
        let list = self.group_dao.find_all().await?;
        for item in list {
            let existing = item.read();
            if existing.name != name {
                continue;
            }
            if let Some(exclude) = exclude_id {
                if existing.id == exclude {
                    continue;
                }
            }
            return Err(ResultCode::GroupNameExists.into());
        }
        Ok(())
    }
}

#[async_trait]
impl BaseSync for GroupService {
    fn sync_type(&self) -> i8 {
        2
    }

    fn merged(&self, merged: Vec<Value>, cloud: &[Value], local: &[Value], op: i8) -> anyhow::Result<Vec<Value>> {
        let _ = (cloud, local, op);
        service_base_sync::keep_referenced(merged, "name", &self.group_2_device_service.referenced_group_ids())
    }

    async fn export_current(&self) -> anyhow::Result<Value> {
        service_base_sync::export_current(self).await
    }

    async fn import_cloud(&self, data: Value) -> anyhow::Result<()> {
        service_base_sync::import_cloud(self, data, None).await
    }
}

submit_sync_entry!(GroupService);
