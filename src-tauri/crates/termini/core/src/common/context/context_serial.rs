use crate::model::dto::shared_ptr::Shared;
use crate::model::config_serial::ConfigSerial;

#[derive(Clone)]
pub struct ContextSerial {
    pub config: Shared<ConfigSerial>,
}
