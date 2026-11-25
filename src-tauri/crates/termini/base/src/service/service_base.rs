use anyhow::Result;
use async_trait::async_trait;
use crate::dao::dao_base::BaseDao;
use termini_core::model::base_target::BaseTarget;
use termini_core::model::dto::shared_ptr::Shared;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, PrimaryKeyTrait};

#[async_trait]
pub trait BaseService: Send + Sync {
    type Entity: EntityTrait;

    fn get_dao(&self) -> Result<Box<dyn BaseDao<Entity = Self::Entity>>>;

    async fn find_all(&self) -> Result<Vec<Shared<<Self::Entity as EntityTrait>::Model>>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + Send + Sync,
    {
        let dao = self.get_dao()?;
        dao.find_all().await
    }

    async fn find_by_id(&self, id: String) -> Result<Option<Shared<<Self::Entity as EntityTrait>::Model>>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + Send + Sync,
        <<Self::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType: From<String>,
    {
        let dao = self.get_dao()?;
        dao.find_by_id(id).await
    }

    async fn insert(&self, entity: <Self::Entity as EntityTrait>::Model) -> Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let dao = self.get_dao()?;
        dao.insert(entity).await
    }

    async fn insert_batch(&self, entities: Vec<<Self::Entity as EntityTrait>::Model>) -> Result<Vec<Shared<<Self::Entity as EntityTrait>::Model>>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let dao = self.get_dao()?;
        dao.insert_batch(entities).await
    }

    async fn update(&self, entity: <Self::Entity as EntityTrait>::Model) -> Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let dao = self.get_dao()?;
        dao.update(entity).await
    }

    async fn delete_by_id(&self, id: String) -> Result<u64>
    where
        <<Self::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType: From<String> + Send,
    {
        let dao = self.get_dao()?;
        dao.delete_by_id(id).await
    }
}
