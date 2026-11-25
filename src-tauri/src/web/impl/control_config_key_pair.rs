use summer::plugin::Service;
use termini_macros::base_control;
use crate::web::control_base::BaseControl;
use crate::model::config_key_pair::ConfigKeyPair;
use crate::service::service_config_key_pair::ConfigKeyPairService;

#[base_control(ConfigKeyPair, prefix = "/config_key_pair")]
#[derive(Clone, Service)]
pub struct ConfigKeyPairController {
    #[inject(component)]
    pub svc: ConfigKeyPairService,
}

impl BaseControl for ConfigKeyPairController {
    type Service = ConfigKeyPairService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}
