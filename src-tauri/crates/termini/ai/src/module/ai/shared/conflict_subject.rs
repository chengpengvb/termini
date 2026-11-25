use serde_json::Value;
use crate::util::hash_util::sha256_hex;

const LOCAL_DEVICE_ID: &str = "__local__";
const EXCLUDED_KEYS: [&str; 4] = ["device_id", "device_ids", "internal_call_id", "__internal_call_id"];

#[derive(Debug, Clone)]
pub struct ConflictSubject {
    pub command_hash: String,
    pub command_text: String,
    pub device_ids: Vec<String>,
    pub shell_commands: Vec<String>, // 只有真正要执行的命令才参与危险关键字匹配, 文件正文/路径不算; 批量时每条都在里面
}

impl ConflictSubject {
    pub fn from_args(args: &Value) -> Self {
        let device_ids = Self::extract_device_ids(args);
        let shell_commands = Self::extract_shell_commands(args);
        let command_text = Self::extract_command_text(args, &shell_commands);
        let command_hash = sha256_hex(&Self::canonical(args));
        Self { device_ids, command_hash, command_text, shell_commands }
    }

    fn extract_shell_commands(args: &Value) -> Vec<String> {
        let Value::Object(map) = args else {
            return Vec::new();
        };
        if let Some(Value::Array(items)) = map.get("commands") {
            return items.iter().filter_map(|item| item.get("command")).filter_map(Value::as_str).filter(|text| !text.trim().is_empty()).map(str::to_string).collect();
        }
        match map.get("command") {
            Some(Value::String(text)) if !text.trim().is_empty() => vec![text.clone()],
            _ => Vec::new(),
        }
    }

    pub fn match_key(&self, tool_id: &str) -> String {
        let mut devices = self.device_ids.clone();
        devices.sort();
        format!("{}|{}|{}", tool_id, devices.join(","), self.command_hash)
    }

    pub fn session_prefix_key(&self, tool_id: &str, prefix: Option<&str>) -> String {
        let mut devices = self.device_ids.clone();
        devices.sort();
        format!("{}|{}|{}", tool_id, devices.join(","), prefix.unwrap_or("*"))
    }

    pub fn allow_device_ids(&self) -> Vec<String> {
        if self.device_ids.is_empty() {
            return vec![LOCAL_DEVICE_ID.to_string()];
        }
        self.device_ids.clone()
    }

    fn extract_device_ids(args: &Value) -> Vec<String> {
        let Value::Object(map) = args else {
            return Vec::new();
        };
        if let Some(Value::String(device_id)) = map.get("device_id") {
            if !device_id.is_empty() {
                return vec![device_id.clone()];
            }
        }
        if let Some(Value::Array(items)) = map.get("device_ids") {
            return items.iter().filter_map(|item| item.as_str()).filter(|item| !item.is_empty()).map(|item| item.to_string()).collect();
        }
        Vec::new()
    }

    fn extract_command_text(args: &Value, shell_commands: &[String]) -> String {
        if shell_commands.len() > 1 {
            return shell_commands.join("\n");
        }
        let Value::Object(map) = args else {
            return args.to_string();
        };
        for key in ["command", "url", "path", "remote_path"] {
            if let Some(Value::String(text)) = map.get(key) {
                if !text.trim().is_empty() {
                    return text.clone();
                }
            }
        }
        args.to_string()
    }

    fn canonical(value: &Value) -> String {
        match value {
            Value::Object(map) => {
                let mut keys: Vec<&String> = map.keys().filter(|key| !EXCLUDED_KEYS.contains(&key.as_str())).collect();
                keys.sort();
                let parts: Vec<String> = keys.iter().filter_map(|key| map.get(*key).map(|item| format!("{}:{}", key, Self::canonical(item)))).collect();
                format!("{{{}}}", parts.join(","))
            }
            Value::Array(items) => {
                let parts: Vec<String> = items.iter().map(Self::canonical).collect();
                format!("[{}]", parts.join(","))
            }
            Value::String(text) => Value::String(text.replace("\r\n", "\n")).to_string(),
            other => other.to_string(),
        }
    }
}
