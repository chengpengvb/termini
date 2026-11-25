use crate::model::device;
use summer::plugin::Service;
use termini_macros::base_dao;

#[base_dao(device::Entity, sort = 120, phase = 30, cache_all = true)]
#[derive(Clone, Service)]
pub struct DeviceDao {}
