use summer::plugin::Service;
use termini_macros::base_dao;
use crate::model::config_serial;
use crate::dao::dao_base::BaseDao;
use crate::model::dto::shared_ptr::Shared;
use crate::model::config_serial::ConfigSerial;

#[base_dao(config_serial::Entity, sort = 120, phase = 30, cache_all = true)]
#[derive(Clone, Service)]
pub struct ConfigSerialDao {}

impl ConfigSerialDao {
    pub fn get_by_port_name(&self, port_name: &str) -> Option<Shared<ConfigSerial>> {
        for entry in self.cache_map().iter() {
            let ptr = entry.value();
            if ptr.read().port_name.as_str() == port_name {
                return Some(ptr.clone());
            }
        }
        None
    }
}
