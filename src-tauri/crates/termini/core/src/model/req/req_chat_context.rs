use serde::Deserialize;

#[repr(i16)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum ChatTargetType {
    Device = 0,
    Group = 1,
}

impl ChatTargetType {
    pub fn as_i16(self) -> i16 {
        self as i16
    }
}

#[repr(i8)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum ConfirmMode {
    #[default]
    Manual = 0,
    Auto = 1,
    Silent = 2,
}

impl ConfirmMode {
    pub fn from_i8(value: i8) -> Option<Self> {
        match value {
            0 => Some(ConfirmMode::Manual),
            1 => Some(ConfirmMode::Auto),
            2 => Some(ConfirmMode::Silent),
            _ => None,
        }
    }

    pub fn prompt_hint(self) -> &'static str {
        match self {
            ConfirmMode::Manual => "请求批准——每个工具操作都会逐条弹给用户点允许或跳过。",
            ConfirmMode::Auto => "替我审批——只有审核模型判定有风险的操作才请求批准，其余自动执行。",
            ConfirmMode::Silent => "完全放行——普通操作不再逐条询问；命中危险命令名单（如 rm 等）仍会请求确认，无人值守时才跳过。",
        }
    }
}

#[repr(i16)]
#[derive(Clone, Copy, Eq, PartialEq, Debug)]
pub enum ChatOrigin {
    User = 0,     //用户在输入框发起
    Answer = 1,   //用户对 ask_user 提问的作答
    Wake = 2,     //后台唤醒源(后台命令/循环监控)投递信号
    Plan = 3,     //计划任务触发
    Alert = 4,    //监控告警触发
    Discover = 5, //指标发现触发
    Continue = 6, //继续
}

impl ChatOrigin {
    pub fn as_i16(self) -> i16 {
        self as i16
    }

    pub fn echo_user(self) -> bool {
        match self {
            ChatOrigin::User | ChatOrigin::Plan | ChatOrigin::Alert | ChatOrigin::Discover => true,
            ChatOrigin::Answer | ChatOrigin::Wake | ChatOrigin::Continue => false,
        }
    }

    pub fn resume_scene(self) -> bool {
        match self {
            ChatOrigin::Answer | ChatOrigin::Wake | ChatOrigin::Continue => true,
            ChatOrigin::User | ChatOrigin::Plan | ChatOrigin::Alert | ChatOrigin::Discover => false,
        }
    }

    pub fn attended(self) -> bool {
        match self {
            ChatOrigin::User | ChatOrigin::Answer | ChatOrigin::Continue => true,
            ChatOrigin::Wake | ChatOrigin::Plan | ChatOrigin::Alert | ChatOrigin::Discover => false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqChatImage {
    pub data: String,       //base64 内容，不含 data: 前缀
    pub media_type: String, //图片 mime，例如 image/png
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReqChatContextPayload {
    pub message: String,       //用户输入的消息
    pub ai_chat_id: String,    //用户选择的ai聊天id
    pub ai_provide_id: String, //用户选择的ai提供商id

    #[serde(default)]
    pub images: Vec<ReqChatImage>, //用户附带的图片

    #[serde(default)]
    pub confirm_mode: i8, //工具执行确认方式，见 ConfirmMode

    #[serde(default)]
    pub thinking_level: Option<String>, //思考挡位名, 对应 AiProvide.thinking_levels.levels 的键; 空用模型默认
}

#[derive(Debug, Clone)]
pub struct ReqChatContext {
    pub message: String,                //用户输入的消息
    pub ai_chat_id: String,             //用户选择的ai聊天id，同时也是推送房间号
    pub origin: ChatOrigin,             //本轮由谁发起, 决定是否回显、是否还原现场
    pub ai_provide_id: String,          //用户选择的ai提供商id
    pub images: Vec<ReqChatImage>,      //用户附带的图片
    pub confirm_mode: ConfirmMode,      //工具执行确认方式
    pub thinking_level: Option<String>, //思考挡位
}

impl ReqChatContext {
    pub fn from_payload(payload: ReqChatContextPayload) -> Self {
        let confirm_mode = ConfirmMode::from_i8(payload.confirm_mode).unwrap_or_default();
        let thinking_level = payload.thinking_level.map(|level| level.trim().to_string()).filter(|level| !level.is_empty());
        Self { message: payload.message, ai_chat_id: payload.ai_chat_id, ai_provide_id: payload.ai_provide_id, images: payload.images, confirm_mode, thinking_level, origin: ChatOrigin::User }
    }
}
