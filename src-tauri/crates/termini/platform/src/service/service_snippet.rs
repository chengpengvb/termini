use crate::model::snippet;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_service;
use crate::dao::dao_base::BaseDao;
use crate::dao::dao_snippet::SnippetDao;
use crate::service::service_base::BaseService;

#[base_service(snippet::Entity, dao = "snippet_dao", sync = 16)]
#[derive(Clone, Service)]
pub struct SnippetService {
    #[inject(component)]
    snippet_dao: SnippetDao,
}

#[async_trait]
impl BaseService for SnippetService {
    type Entity = snippet::Entity;

    fn get_dao(&self) -> anyhow::Result<Box<dyn BaseDao<Entity = Self::Entity>>> {
        Ok(Box::new(self.snippet_dao.clone()))
    }
}
