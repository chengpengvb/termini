pub mod pdf;
pub mod web;
pub mod task;
pub mod wake;
pub mod image;
pub mod shell;
pub mod remote;
pub mod document;
pub mod tool_task;
pub mod tool_group;
pub mod tool_ftp_fs;
pub mod edit_replace;
pub mod tool_sftp_fs;
pub mod tool_ask_user;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod tool_code_map;
pub mod tool_log_list;
pub mod tool_mcp_list;
pub mod tool_pdf_read;
pub mod tool_docx_read;
pub mod tool_edit_hint;
pub mod tool_file_read;
pub mod tool_mail_send;
pub mod tool_pdf_write;
pub mod tool_web_fetch;
pub mod tool_xlsx_read;
pub mod tool_docx_write;
pub mod tool_image_read;
pub mod tool_mcp_create;
pub mod tool_mcp_delete;
pub mod tool_mcp_update;
pub mod tool_script_run;
pub mod tool_skill_list;
pub mod tool_todo_write;
pub mod tool_web_search;
pub mod tool_xlsx_write;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod tool_code_search;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod tool_code_symbol;
pub mod tool_device_list;
pub mod tool_entity_call;
pub mod tool_entity_list;
pub mod tool_output_read;
pub mod tool_tool_search;
pub mod tool_chat_history;
pub mod tool_derived_list;
pub mod tool_skill_create;
pub mod tool_skill_update;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod tool_code_semantic;
pub mod tool_device_import;
pub mod tool_entity_notify;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod tool_desktop_action;
pub mod tool_image_generate;
pub mod tool_search_replace;
pub mod tool_sftp_file_read;
pub mod tool_entity_describe;
pub mod tool_ssh_shell_batch;
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub mod tool_desktop_snapshot;
pub mod tool_ftp_transfer_batch;
pub mod tool_telnet_shell_batch;
pub mod tool_sftp_search_replace;
pub mod tool_sftp_transfer_batch;
pub mod tool_device_profile_update;
