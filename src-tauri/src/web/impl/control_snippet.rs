use summer::plugin::Service;
use termini_macros::base_control;
use crate::model::snippet::Snippet;
use crate::web::control_base::BaseControl;
use crate::service::service_snippet::SnippetService;

#[base_control(Snippet, prefix = "/snippet")]
#[derive(Clone, Service)]
pub struct SnippetController {
    #[inject(component)]
    svc: SnippetService,
}

impl BaseControl for SnippetController {
    type Service = SnippetService;

    fn get_service(&self) -> &Self::Service {
        &self.svc
    }
}
