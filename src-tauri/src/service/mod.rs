pub mod service_store;
pub use termini_ai::service::service_agent;
pub use termini_base::service::service_base;
pub use termini_ai::service::service_ai_chat;
pub use termini_remote::service::service_ssh;
pub use termini_ai::service::service_ai_skill;
pub use termini_remote::service::service_sftp;
pub use termini_ai::service::service_ai_project;
pub use termini_ai::service::service_ai_provide;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use termini_ai::service::service_code_index;
pub use termini_ai::service::service_ai_plan_run;
pub use termini_platform::service::service_group;
pub use termini_platform::service::service_shell;
pub use termini_ai::service::service_ai_chat_item;
pub use termini_ai::service::service_ai_plan_task;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use termini_ai::service::service_job_discover;
pub use termini_platform::service::service_config;
pub use termini_platform::service::service_device;
pub use termini_ai::service::service_ai_mcp_server;
pub use termini_platform::service::service_snippet;
pub use termini_remote::service::service_sftp_drag;
pub use termini_remote::service::service_ssh_shell;
pub use termini_ai::service::service_ai_file_change;
pub use termini_platform::service::service_base_api;
pub use termini_ai::service::service_command_suggest;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use termini_collect::service::service_job_device;
pub use termini_platform::service::service_file_edit;
pub use termini_platform::service::service_cloud_sync;
pub use termini_ai::service::service_ai_chat_2_project;
pub use termini_platform::service::service_diagnostics;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use termini_collect::service::service_job_indicator;
pub use termini_platform::service::service_command_rule;
pub use termini_platform::service::service_port_forward;
pub use termini_platform::service::service_account_token;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use termini_platform::service::service_config_serial;
pub use termini_platform::service::service_device_import;
pub use termini_platform::service::service_shell_history;
pub use termini_platform::service::service_group_2_device;
pub use termini_platform::service::service_session_record;
pub use termini_platform::service::service_config_key_pair;
pub use termini_platform::service::service_impl_system_util;
pub use termini_platform::service::service_job_indicator_alert;
pub use termini_platform::service::service_job_indicator_value;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use termini_collect::service::service_job_device_2_indicator;