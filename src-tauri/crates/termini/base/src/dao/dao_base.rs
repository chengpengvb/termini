use std::sync::Arc;
use dashmap::DashMap;
use anyhow::{bail, Result};
use async_trait::async_trait;
use termini_core::util::id_factory::uuid;
use termini_core::model::base_target::BaseTarget;
use termini_core::model::dto::shared_ptr::Shared;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, PrimaryKeyTrait};

pub fn new_cache_map<M>() -> Arc<DashMap<String, Shared<M>>> {
    Arc::new(DashMap::new())
}

#[async_trait]
pub trait BaseDao: Send + Sync {
    type Entity: EntityTrait;

    fn cache_all(&self) -> bool;

    fn get_conn(&self) -> Result<DatabaseConnection>; //向子类获取连接

    fn cache_map(&self) -> &DashMap<String, Shared<<Self::Entity as EntityTrait>::Model>>;

    async fn init(&self) -> Result<()>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        use sea_orm::{ConnectionTrait, Schema};
        let conn = self.get_conn()?;
        let backend = conn.get_database_backend();
        let schema = Schema::new(backend);
        let mut stmt = schema.create_table_from_entity(<Self::Entity as Default>::default());
        stmt.if_not_exists();
        conn.execute(backend.build(&stmt)).await?;
        self.load().await?;
        Ok(())
    }

    async fn load(&self) -> Result<()>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + Send + Sync,
    {
        if !self.cache_all() {
            return Ok(());
        }
        let conn = self.get_conn()?;
        let cache = self.cache_map();
        let results = Self::Entity::find().all(&conn).await?;
        cache.clear();
        for mut model in results {
            model.decrypt_sensitive();
            let Some(id) = model.get_id() else {
                bail!("load got model with empty id");
            };
            cache.insert(id, Shared::new(model));
        }
        Ok(())
    }

    async fn find_all(&self) -> Result<Vec<Shared<<Self::Entity as EntityTrait>::Model>>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + Send + Sync,
    {
        if self.cache_all() {
            return Ok(self.cache_map().iter().map(|e| e.value().clone()).collect());
        }
        let conn = self.get_conn()?;
        let results = Self::Entity::find().all(&conn).await?;
        Ok(results
            .into_iter()
            .map(|mut model| {
                model.decrypt_sensitive();
                Shared::new(model)
            })
            .collect())
    }

    async fn find_by_id(&self, id: String) -> Result<Option<Shared<<Self::Entity as EntityTrait>::Model>>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + Send + Sync,
        <<Self::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType: From<String>,
    {
        if self.cache_all() {
            return Ok(self.cache_map().get(&id).map(|v| v.value().clone()));
        }
        let conn = self.get_conn()?;
        let pk_val = <<Self::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType::from(id);
        let result = Self::Entity::find_by_id(pk_val).one(&conn).await?;
        Ok(result.map(|mut model| {
            model.decrypt_sensitive();
            Shared::new(model)
        }))
    }

    async fn insert(&self, mut entity: <Self::Entity as EntityTrait>::Model) -> Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        match entity.get_id() {
            Some(v) if !v.is_empty() => v,
            _ => {
                let next = uuid();
                entity.set_id(next.clone());
                next
            }
        };
        entity.encrypt_sensitive();
        let conn = self.get_conn()?;
        let active = entity.into_active_model();
        let mut result = active.insert(&conn).await?;
        result.decrypt_sensitive();
        if !self.cache_all() {
            return Ok(Shared::new(result));
        }
        let Some(id) = result.get_id() else {
            bail!("insert got model with empty id");
        };
        let shared = Shared::new(result);
        self.cache_map().insert(id, shared.clone());
        Ok(shared)
    }

    async fn insert_batch(&self, mut entities: Vec<<Self::Entity as EntityTrait>::Model>) -> Result<Vec<Shared<<Self::Entity as EntityTrait>::Model>>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        if entities.is_empty() {
            return Ok(Vec::new());
        }
        for entity in &mut entities {
            match entity.get_id() {
                Some(v) if !v.is_empty() => {}
                _ => {
                    entity.set_id(uuid());
                }
            }
            entity.encrypt_sensitive();
        }
        let conn = self.get_conn()?;
        let active_models: Vec<<Self::Entity as EntityTrait>::ActiveModel> = entities.iter().cloned().map(|entity| entity.into_active_model()).collect();
        Self::Entity::insert_many(active_models).exec(&conn).await?;
        let mut shared_list = Vec::with_capacity(entities.len());
        for mut entity in entities {
            entity.decrypt_sensitive();
            let Some(id) = entity.get_id() else {
                bail!("insert_batch got model with empty id");
            };
            let shared = Shared::new(entity);
            if self.cache_all() {
                self.cache_map().insert(id, shared.clone());
            }
            shared_list.push(shared);
        }
        Ok(shared_list)
    }

    async fn update(&self, mut entity: <Self::Entity as EntityTrait>::Model) -> Result<Shared<<Self::Entity as EntityTrait>::Model>>
    where
        <Self::Entity as EntityTrait>::Model: BaseTarget + IntoActiveModel<<Self::Entity as EntityTrait>::ActiveModel> + Send + Sync,
        <Self::Entity as EntityTrait>::ActiveModel: ActiveModelTrait<Entity = Self::Entity> + Send,
    {
        let conn = self.get_conn()?;
        match entity.get_id() {
            Some(v) if !v.is_empty() => {}
            _ => {
                bail!("update requires non-empty id");
            }
        }
        entity.encrypt_sensitive();
        let mut result = entity.into_active_model().reset_all().update(&conn).await?;
        result.decrypt_sensitive();
        if !self.cache_all() {
            return Ok(Shared::new(result));
        }
        let Some(id) = result.get_id() else {
            bail!("update got model with empty id");
        };
        let Some(existing) = self.cache_map().get(&id) else {
            bail!("update target not found in cache for id: {}", id);
        };
        *existing.value().write() = result;
        Ok(existing.value().clone())
    }

    async fn delete_by_id(&self, id: String) -> Result<u64>
    where
        <<Self::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType: From<String> + Send,
    {
        let conn = self.get_conn()?;
        let pk_val = <<Self::Entity as EntityTrait>::PrimaryKey as PrimaryKeyTrait>::ValueType::from(id.clone());
        let result = Self::Entity::delete_by_id(pk_val).exec(&conn).await?;
        if self.cache_all() {
            self.cache_map().remove(&id);
        }
        Ok(result.rows_affected)
    }
}
