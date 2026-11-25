pub mod dao;
pub mod module;
pub mod service;

pub mod base {
    pub use termini_base::base::config;
    pub use termini_base::base::base_app;
}

pub mod system {
    pub use termini_base::system::{bootstrap, system_app};
}

pub mod util {
    pub use termini_remote::util::*;
}

pub mod web {
    pub use termini_remote::web::ws;
}

pub use termini_core::model;