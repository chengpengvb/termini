pub mod collect;
pub mod service;

pub mod base {
    pub use termini_base::base::config;
    pub use termini_base::base::base_app;
}

pub mod system {
    pub use termini_base::system::{bootstrap, system_app};
}

pub mod dao {
    pub use termini_platform::dao::*;
    pub use termini_base::dao::dao_base;
}

pub mod util {
    pub use termini_remote::util::*;
}

pub mod module {
    pub use termini_core::common;
    pub use termini_platform::module::shell;
    pub use termini_remote::module::{rdp, sftp, ssh, ssh_forward, ssh_shell, telnet, telnet_shell};
}

pub mod web {
    pub use termini_platform::web::ws;
}

pub use termini_core::model;
