use serde::Serialize;
use sea_orm::EntityTrait;
use async_trait::async_trait;
use crate::service::service_base::BaseService;
use termini_core::model::base_target::BaseTarget;
use termini_core::model::dto::shared_ptr::Shared;
use termini_core::model::resp::resp_model::ResultModel;

type ServiceEntity<S> = <S as BaseService>::Entity;
type ServiceModel<S> = <<S as BaseService>::Entity as EntityTrait>::Model;

#[async_trait]
pub trait BaseControl: Send + Sync {
    type Service: BaseService;

    fn get_service(&self) -> &Self::Service;

    async fn select_all(&self) -> ResultModel<Vec<Shared<ServiceModel<Self::Service>>>>
    where
        ServiceModel<Self::Service>: BaseTarget + Serialize + Send + Sync,
    {
        ResultModel::from_result(self.get_service().find_all().await)
    }

    async fn select(&self, id: String) -> ResultModel<Option<Shared<ServiceModel<Self::Service>>>>
    where
        ServiceModel<Self::Service>: BaseTarget + Serialize + Send + Sync,
        <<ServiceEntity<Self::Service> as EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<String> + Send,
    {
        ResultModel::from_result(self.get_service().find_by_id(id).await)
    }

    async fn insert(&self, body: ServiceModel<Self::Service>) -> ResultModel<Shared<ServiceModel<Self::Service>>>
    where
        ServiceModel<Self::Service>: BaseTarget + Serialize + sea_orm::IntoActiveModel<<ServiceEntity<Self::Service> as EntityTrait>::ActiveModel> + Send + Sync,
        <ServiceEntity<Self::Service> as EntityTrait>::ActiveModel: sea_orm::ActiveModelTrait<Entity = ServiceEntity<Self::Service>> + Send,
    {
        ResultModel::from_result(self.get_service().insert(body).await)
    }

    async fn insert_batch(&self, body: Vec<ServiceModel<Self::Service>>) -> ResultModel<Vec<Shared<ServiceModel<Self::Service>>>>
    where
        ServiceModel<Self::Service>: BaseTarget + Serialize + sea_orm::IntoActiveModel<<ServiceEntity<Self::Service> as EntityTrait>::ActiveModel> + Send + Sync,
        <ServiceEntity<Self::Service> as EntityTrait>::ActiveModel: sea_orm::ActiveModelTrait<Entity = ServiceEntity<Self::Service>> + Send,
    {
        ResultModel::from_result(self.get_service().insert_batch(body).await)
    }

    async fn update(&self, body: ServiceModel<Self::Service>) -> ResultModel<Shared<ServiceModel<Self::Service>>>
    where
        ServiceModel<Self::Service>: BaseTarget + Serialize + sea_orm::IntoActiveModel<<ServiceEntity<Self::Service> as EntityTrait>::ActiveModel> + Send + Sync,
        <ServiceEntity<Self::Service> as EntityTrait>::ActiveModel: sea_orm::ActiveModelTrait<Entity = ServiceEntity<Self::Service>> + Send,
    {
        ResultModel::from_result(self.get_service().update(body).await)
    }

    async fn delete_by_id(&self, id: String) -> ResultModel<()>
    where
        <<ServiceEntity<Self::Service> as EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType: From<String> + Send,
    {
        ResultModel::from_result(self.get_service().delete_by_id(id).await.map(|_| ()))
    }
}
