pub mod service_agent;
pub mod service_ai_chat;
pub mod service_ai_skill;
pub mod service_ai_project;
pub mod service_ai_provide;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod service_code_index;
pub mod service_ai_plan_run;
pub mod service_ai_chat_item;
pub mod service_ai_plan_task;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod service_job_discover;
pub mod service_ai_mcp_server;
pub mod service_ai_chat_memory;
pub mod service_ai_file_change;
pub mod service_command_suggest;
pub mod service_ai_chat_2_project;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub use termini_collect::service::*;
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub use termini_platform::service::*;
pub mod service_ai_chat_memory_snapshot;
pub use termini_base::service::service_base;
pub use termini_base::service::service_ai_crud;
pub use termini_base::service::service_base_sync;