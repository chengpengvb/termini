use anyhow::anyhow;
use crate::module::common::context::context_shell::ShellDescr;
use crate::module::shell::platform::shell_discover::ShellDiscover;

pub const IOS_SHELL_ID: &str = "ios.system";

pub struct ShellDiscoverIos;

impl ShellDiscoverIos {
    pub fn new() -> Self {
        Self
    }
}

impl ShellDiscover for ShellDiscoverIos {
    fn discover(&self) -> Vec<ShellDescr> {
        vec![ShellDescr {
            shell_id: IOS_SHELL_ID.to_owned(),
            description: "iOS 应用内命令环境 (ios_system)：逐条执行命令，无真实 PTY，不支持全屏 TUI、REPL 与后台常驻进程".to_owned(),
            exe: None,
            args: Vec::new(),
            env: Vec::new(),
            supports_osc633: false,
        }]
    }

    fn build_exec_args(&self, _descr: &ShellDescr, _command: &str) -> anyhow::Result<Vec<String>> {
        Err(anyhow!("ios_system runs in-process and cannot be launched as a child process"))
    }
}
