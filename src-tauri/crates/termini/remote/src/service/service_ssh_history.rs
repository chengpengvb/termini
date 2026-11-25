use std::time::Duration;
use summer::tracing::error;
use summer::plugin::Service;
use async_trait::async_trait;
use crate::util::id_factory::uuid;
use crate::base::base_app::{AppContext, BaseApp};
use crate::service::service_ssh_shell::SshShellService;
use crate::util::ssh_shell::ssh_shell_const::READ_HISTORY_COMMAND;

const HISTORY_EXEC_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone, Service)]
pub struct SshHistoryService {
    #[inject(component)]
    ssh_shell_service: SshShellService,
}

#[async_trait]
impl BaseApp for SshHistoryService {
    fn name(&self) -> &'static str {
        "SshHistoryService"
    }

    async fn start(&self, _ctx: &AppContext) -> anyhow::Result<()> {
        Ok(())
    }
}

impl SshHistoryService {
    pub async fn get_cmd_history(&self, device_id: String) -> Vec<String> {
        let session_id = uuid();
        if let Err(e) = self.ssh_shell_service.open(session_id.clone(), device_id).await {
            error!("[ssh] get_cmd_history open error: {}", e);
            return vec![];
        }
        let history = match self.ssh_shell_service.exec(&session_id, READ_HISTORY_COMMAND.to_string(), HISTORY_EXEC_TIMEOUT).await {
            Ok((_exit_code, output)) => Self::parse_history_output(output),
            Err(e) => {
                error!("[ssh] get_cmd_history exec error: {}", e);
                vec![]
            }
        };
        if let Err(e) = self.ssh_shell_service.close(&session_id).await {
            error!("[ssh] get_cmd_history close error: {}", e);
        }
        history
    }

    fn parse_history_output(output: String) -> Vec<String> {
        output.lines().map(|line| line.trim_end().to_string()).filter(|line| !line.is_empty()).collect()
    }
}
