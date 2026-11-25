use summer::plugin::Service;
use termini_macros::base_dao;
use crate::model::session_record;

#[base_dao(session_record::Entity, sort = 130, phase = 30, cache_all = false)]
#[derive(Clone, Service)]
pub struct SessionRecordDao {}
