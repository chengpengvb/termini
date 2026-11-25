use std::time::Duration;
use serde::{Deserialize, Serialize};
use termini_macros::{base_target, Redact};

pub const DEFAULT_CONFIG_ID: &str = "000000";

pub const SUGGEST_COUNT_MIN: i16 = 1;
pub const SUGGEST_COUNT_MAX: i16 = 10;
pub const SUGGEST_COUNT_DEFAULT: i16 = 3;

pub const JOB_INDICATOR_VALUE_CACHE_SIZE_MIN: i16 = 12;
pub const JOB_INDICATOR_VALUE_CACHE_SIZE_MAX: i16 = 500;
pub const JOB_INDICATOR_VALUE_CACHE_SIZE_DEFAULT: i16 = 36;

pub const JOB_EXEC_TIMEOUT_SECS_MIN: i64 = 5;
pub const JOB_EXEC_TIMEOUT_SECS_MAX: i64 = 300;
pub const JOB_EXEC_TIMEOUT_SECS_DEFAULT: i64 = 30;

pub const JOB_WORKER_PARALLELISM_MIN: i16 = 1;
pub const JOB_WORKER_PARALLELISM_MAX: i16 = 200;
pub const JOB_WORKER_PARALLELISM_DEFAULT: i16 = 25;

pub const JOB_SSH_POOL_MAX_SIZE_MIN: i16 = 1;
pub const JOB_SSH_POOL_MAX_SIZE_MAX: i16 = 100;
pub const JOB_SSH_POOL_MAX_SIZE_DEFAULT: i16 = 15;

pub const JOB_SSH_POOL_IDLE_SECS_MIN: i64 = 5;
pub const JOB_SSH_POOL_IDLE_SECS_MAX: i64 = 600;
pub const JOB_SSH_POOL_IDLE_SECS_DEFAULT: i64 = 15;

pub const JOB_DISCOVER_PARALLELISM_MIN: i16 = 1;
pub const JOB_DISCOVER_PARALLELISM_MAX: i16 = 10;
pub const JOB_DISCOVER_PARALLELISM_DEFAULT: i16 = 2;

pub const SHELL_CPU_PERCENT_MIN: i16 = 10;
pub const SHELL_CPU_PERCENT_MAX: i16 = 100;
pub const SHELL_CPU_PERCENT_DEFAULT: i16 = 80;

pub const CLOSE_ACTION_ASK: i16 = 0;
pub const CLOSE_ACTION_EXIT: i16 = 1;
pub const CLOSE_ACTION_TRAY: i16 = 2;

pub const WEBHOOK_KIND_DINGTALK: i16 = 1;
pub const WEBHOOK_KIND_WECOM: i16 = 2;
pub const WEBHOOK_KIND_FEISHU: i16 = 3;
pub const WEBHOOK_KIND_TELEGRAM: i16 = 4;

pub const WEB_FETCH_TRUSTED_HOSTS_DEFAULT: &str = "github.com raw.githubusercontent.com gitlab.com stackoverflow.com developer.mozilla.org docs.rs crates.io doc.rust-lang.org rust-lang.org docs.python.org pypi.org npmjs.com nodejs.org learn.microsoft.com docs.microsoft.com developer.apple.com kubernetes.io docs.docker.com man7.org wikipedia.org cloud.google.com docs.aws.amazon.com developer.hashicorp.com nginx.org redis.io postgresql.org mysql.com ubuntu.com debian.org archlinux.org kernel.org openssh.com cisco.com huawei.com h3c.com juniper.net go.dev";
pub const CONFIRM_PREFIX_SKIP_TOKENS_DEFAULT: &str = "sudo nohup time env exec";
pub const CONFIRM_PREFIX_TOOLS_DEFAULT: &str = "git docker docker-compose kubectl helm systemctl journalctl npm yarn pnpm cargo apt apt-get yum dnf pip pip3 brew terraform gcloud aws az podman nmcli";

pub const MODEL_ROLE_VISION: i16 = 0;
pub const MODEL_ROLE_IMAGE_GEN: i16 = 1;
pub const MODEL_ROLE_EMBEDDING: i16 = 2;
pub const MODEL_ROLE_ASSIST: i16 = 3;

#[repr(i16)]
#[derive(Clone, Copy, Eq, PartialEq, Debug, Default)]
pub enum SystemLanguage {
    ZhCn = 0,
    #[default]
    EnUs = 1,
    JaJp = 2,
    KoKr = 3,
    DeDe = 4,
    FrFr = 5,
    EsEs = 6,
    ItIt = 7,
    PtPt = 8,
    NlNl = 9,
    ZhTw = 10,
    HiIn = 11,
}

impl SystemLanguage {
    pub fn from_i16(value: i16) -> Self {
        match value {
            0 => SystemLanguage::ZhCn,
            2 => SystemLanguage::JaJp,
            3 => SystemLanguage::KoKr,
            4 => SystemLanguage::DeDe,
            5 => SystemLanguage::FrFr,
            6 => SystemLanguage::EsEs,
            7 => SystemLanguage::ItIt,
            8 => SystemLanguage::PtPt,
            9 => SystemLanguage::NlNl,
            10 => SystemLanguage::ZhTw,
            11 => SystemLanguage::HiIn,
            _ => SystemLanguage::EnUs,
        }
    }

    pub fn from_locale(tag: &str) -> Self {
        let primary = tag.trim().split(['.', '@']).next().unwrap_or("").replace('_', "-").to_ascii_lowercase();
        if primary.starts_with("zh-hant") || primary.starts_with("zh-tw") || primary.starts_with("zh-hk") || primary.starts_with("zh-mo") {
            return SystemLanguage::ZhTw;
        }
        if primary.starts_with("zh") {
            return SystemLanguage::ZhCn;
        }
        let lang = primary.split('-').next().unwrap_or("");
        match lang {
            "ja" => SystemLanguage::JaJp,
            "ko" => SystemLanguage::KoKr,
            "de" => SystemLanguage::DeDe,
            "fr" => SystemLanguage::FrFr,
            "es" => SystemLanguage::EsEs,
            "it" => SystemLanguage::ItIt,
            "pt" => SystemLanguage::PtPt,
            "nl" => SystemLanguage::NlNl,
            "hi" => SystemLanguage::HiIn,
            _ => SystemLanguage::EnUs,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            SystemLanguage::ZhCn => "简体中文",
            SystemLanguage::EnUs => "English",
            SystemLanguage::JaJp => "日本語",
            SystemLanguage::KoKr => "한국어",
            SystemLanguage::DeDe => "Deutsch",
            SystemLanguage::FrFr => "Français",
            SystemLanguage::EsEs => "Español",
            SystemLanguage::ItIt => "Italiano",
            SystemLanguage::PtPt => "Português",
            SystemLanguage::NlNl => "Nederlands",
            SystemLanguage::ZhTw => "繁體中文",
            SystemLanguage::HiIn => "हिंदी",
        }
    }
}

#[base_target(table = "config", entity_type = 3)]
#[derive(Clone, Default, Serialize, Deserialize, Redact)]
pub struct Model {
    pub suggest_count: i16, // 候选条数，<=0 按 SUGGEST_COUNT_DEFAULT 取

    pub suggest_enabled: bool, // 终端命令提示开关

    #[sea_orm(ignore)]
    #[serde(default)]
    pub login_url: Option<String>,

    #[serde(default)]
    pub index_include_tests: bool, // 是否把测试目录纳入语义索引

    #[sea_orm(ignore)]
    #[serde(default)]
    pub portal_url: Option<String>,

    #[sea_orm(ignore)]
    #[serde(default)]
    pub embedding_dim: Option<i32>,

    #[sea_orm(ignore)]
    #[serde(default)]
    pub embedding_batch_size: Option<i32>,

    #[sea_orm(ignore)]
    #[serde(default)]
    pub vision_ai_provide_id: Option<String>,

    #[sea_orm(ignore)]
    #[serde(default)]
    pub vision_max_pixels: Option<i32>, // 服务端按所选视觉模型配置的像素上限, 空则不缩放

    #[sea_orm(ignore)]
    #[serde(default)]
    pub vision_jpeg_quality: Option<i32>,

    #[sea_orm(ignore)]
    #[serde(default)]
    pub image_gen_ai_provide_id: Option<String>, // 服务端配置的图片生成模型: image_generate 工具使用

    #[sea_orm(ignore)]
    #[serde(default)]
    pub suggest_ai_provide_id: Option<String>,

    #[sea_orm(ignore)]
    #[serde(default)]
    pub embedding_ai_provide_id: Option<String>,

    #[sea_orm(ignore)]
    #[serde(default)]
    pub assist_ai_provide_id: Option<String>, // 服务端配置的辅助模型: 上下文折叠、意图分流等轻量任务

    #[sea_orm(ignore)]
    #[serde(default)]
    pub tool_mirror_url: Option<String>,

    #[sea_orm(ignore)]
    #[serde(default)]
    pub suggest_free_monthly: Option<i32>,
    #[serde(default = "default_true")]
    pub vision_use_cloud: bool, // 图片读取用云端下发的模型; false 则用 vision_local_ai_provide_id
    #[serde(default)]
    pub vision_local_ai_provide_id: Option<String>,
    #[serde(default = "default_true")]
    pub image_gen_use_cloud: bool,
    #[serde(default)]
    pub image_gen_local_ai_provide_id: Option<String>,
    #[serde(default = "default_true")]
    pub embedding_use_cloud: bool,
    #[serde(default)]
    pub embedding_local_ai_provide_id: Option<String>,
    #[serde(default)]
    pub embedding_local_dim: Option<i32>, // 本地向量模型的维度, 云端模式用 embedding_dim
    #[serde(default = "default_true")]
    pub assist_use_cloud: bool,
    #[serde(default)]
    pub assist_local_ai_provide_id: Option<String>,

    #[serde(default)]
    pub index_ignore: Option<String>, // 换行分隔的 gitignore 语法规则，叠加在内置默认之上

    #[serde(default)]
    pub index_dirs: Option<String>, // 换行分隔的索引目录绝对路径，启动时建索引并监听变更

    pub system_language: i16,           // 系统语言 SystemLanguage，AI 给运维看的结论按它书写
    pub local_shell_id: Option<String>, // 本地shell id

    #[serde(default = "default_true")]
    pub open_here_enabled: bool,

    #[serde(default)]
    pub shell_cpu_percent: i16, // 助手执行的本机命令合计可占的 CPU 上限百分比，<=0 按 SHELL_CPU_PERCENT_DEFAULT 取，100 为不限制

    #[serde(default)]
    pub client_device_id: Option<String>, // 本机稳定设备标识，首次登录时生成并登记到账号中心

    pub danger_words: String,

    #[serde(default)]
    pub web_fetch_trusted_hosts: Option<String>, // 网页抓取免确认的域名(含子域), 空按 WEB_FETCH_TRUSTED_HOSTS_DEFAULT 取
    #[serde(default)]
    pub confirm_prefix_skip_tokens: Option<String>, // 提取命令前缀时跳过的包装词(sudo/env...), 空按 CONFIRM_PREFIX_SKIP_TOKENS_DEFAULT 取
    #[serde(default)]
    pub confirm_prefix_tools: Option<String>, // 前缀要带上子命令的工具(git/docker...), 空按 CONFIRM_PREFIX_TOOLS_DEFAULT 取
    #[serde(default)]
    pub audit_policy: Option<String>, // 替我审批时给审核模型的自定义策略正文, 优先于内置条款

    pub job_exec_timeout_secs: i64,                 // 单条采集命令执行超时（秒），<=0 按 JOB_EXEC_TIMEOUT_SECS_DEFAULT 取
    pub job_worker_parallelism: i16,                // 采集调度并发上限，<=0 按 JOB_WORKER_PARALLELISM_DEFAULT 取
    pub job_ssh_pool_max_size: i16,                 // SSH 锚点池容量上限，<=0 按 JOB_SSH_POOL_MAX_SIZE_DEFAULT 取
    pub job_ssh_pool_idle_secs: i64,                // SSH 锚点空闲多久后关闭（秒），<=0 按 JOB_SSH_POOL_IDLE_SECS_DEFAULT 取
    pub job_discover_parallelism: i16,              // 同时探索的设备数上限，<=0 按 JOB_DISCOVER_PARALLELISM_DEFAULT 取
    pub job_indicator_value_cache_size: i16,        // 每个指标内存缓存样本数，<=0 按 JOB_INDICATOR_VALUE_CACHE_SIZE_DEFAULT 取
    pub job_alert_ai_provide_id: Option<String>,    // 任务告警用的模型；未配置则告警 AI 不启用
    pub job_discover_ai_provide_id: Option<String>, // 指标探索用的模型；未配置则探索功能不可用

    pub redis_db: i16,                  // Redis DB 索引
    pub redis_port: i32,                // Redis 端口，0 按 6379
    pub redis_host: String,             // Redis 主机
    pub redis_enabled: bool,            // 启用 Redis 存储指标值缓存
    pub redis_username: Option<String>, // Redis 用户名（ACL），可空
    #[redact]
    pub redis_password: Option<String>, // Redis 密码，可空
    pub redis_job_indicator_value_cache_size: i16, // 每个指标 Redis 最大条数封顶，防短周期打爆
    pub redis_job_indicator_value_retention_secs: i64, // 每个指标 Redis 保留时长（秒），对齐图表窗口；<=0 仅按条数
    pub notify_enabled: bool,           // 告警邮件通知开关
    pub notify_email: Option<String>,   // 告警通知邮箱，为空则不发
    pub close_action: i16,              // 关闭按钮行为 CLOSE_ACTION_*，0=询问 1=退出 2=托盘
    #[serde(default)]
    pub notify_tray_enabled: bool, // 告警系统通知（托盘弹窗）开关
    #[serde(default)]
    pub notify_webhook_enabled: bool, // 告警 webhook 通知开关
    #[serde(default)]
    pub notify_webhook_kind: i16, // webhook 类型 WEBHOOK_KIND_*
    #[serde(default)]
    #[redact]
    pub notify_webhook_url: Option<String>, // webhook 地址；Telegram 填 https://api.telegram.org/bot<token>/sendMessage
    #[serde(default)]
    #[redact]
    pub notify_webhook_extra: Option<String>, // Telegram 的 chat_id；其他类型不用
    #[serde(default)]
    pub job_heartbeat_at: i64, // 采集心跳（Unix 毫秒），启动时据此算出监控中断了多久
}

fn default_true() -> bool {
    true
}

impl Model {
    fn pick_provide_id(use_cloud: bool, local: &Option<String>, cloud: &Option<String>) -> Option<String> {
        let chosen = if use_cloud { cloud } else { local };
        chosen.clone().filter(|id| !id.trim().is_empty())
    }

    pub fn resolve_vision_provide_id(&self) -> Option<String> {
        Self::pick_provide_id(self.vision_use_cloud, &self.vision_local_ai_provide_id, &self.vision_ai_provide_id)
    }

    pub fn resolve_image_gen_provide_id(&self) -> Option<String> {
        Self::pick_provide_id(self.image_gen_use_cloud, &self.image_gen_local_ai_provide_id, &self.image_gen_ai_provide_id)
    }

    pub fn resolve_embedding_provide_id(&self) -> Option<String> {
        Self::pick_provide_id(self.embedding_use_cloud, &self.embedding_local_ai_provide_id, &self.embedding_ai_provide_id)
    }

    pub fn resolve_embedding_dim(&self) -> Option<i32> {
        let dim = if self.embedding_use_cloud { self.embedding_dim } else { self.embedding_local_dim };
        dim.filter(|value| *value > 0)
    }

    pub fn resolve_assist_provide_id(&self) -> Option<String> {
        Self::pick_provide_id(self.assist_use_cloud, &self.assist_local_ai_provide_id, &self.assist_ai_provide_id)
    }

    pub fn resolve_system_language(&self) -> SystemLanguage {
        SystemLanguage::from_i16(self.system_language)
    }

    pub fn resolve_job_exec_timeout(&self) -> Duration {
        let secs = if self.job_exec_timeout_secs <= 0 { JOB_EXEC_TIMEOUT_SECS_DEFAULT } else { self.job_exec_timeout_secs };
        Duration::from_secs(secs as u64)
    }

    pub fn resolve_job_worker_parallelism(&self) -> usize {
        let size = if self.job_worker_parallelism <= 0 { JOB_WORKER_PARALLELISM_DEFAULT } else { self.job_worker_parallelism };
        size as usize
    }

    pub fn resolve_job_ssh_pool_max_size(&self) -> usize {
        let size = if self.job_ssh_pool_max_size <= 0 { JOB_SSH_POOL_MAX_SIZE_DEFAULT } else { self.job_ssh_pool_max_size };
        size as usize
    }

    pub fn resolve_job_discover_parallelism(&self) -> usize {
        let size = if self.job_discover_parallelism <= 0 { JOB_DISCOVER_PARALLELISM_DEFAULT } else { self.job_discover_parallelism };
        size as usize
    }

    pub fn resolve_job_ssh_pool_idle(&self) -> Duration {
        let secs = if self.job_ssh_pool_idle_secs <= 0 { JOB_SSH_POOL_IDLE_SECS_DEFAULT } else { self.job_ssh_pool_idle_secs };
        Duration::from_secs(secs as u64)
    }

    pub fn resolve_web_fetch_trusted_hosts(&self) -> Vec<String> {
        split_config_list(self.web_fetch_trusted_hosts.as_deref(), WEB_FETCH_TRUSTED_HOSTS_DEFAULT)
    }

    pub fn resolve_confirm_prefix_skip_tokens(&self) -> Vec<String> {
        split_config_list(self.confirm_prefix_skip_tokens.as_deref(), CONFIRM_PREFIX_SKIP_TOKENS_DEFAULT)
    }

    pub fn resolve_confirm_prefix_tools(&self) -> Vec<String> {
        split_config_list(self.confirm_prefix_tools.as_deref(), CONFIRM_PREFIX_TOOLS_DEFAULT)
    }

    pub fn resolve_shell_cpu_percent(&self) -> i16 {
        if self.shell_cpu_percent <= 0 {
            return SHELL_CPU_PERCENT_DEFAULT;
        }
        self.shell_cpu_percent.clamp(SHELL_CPU_PERCENT_MIN, SHELL_CPU_PERCENT_MAX)
    }

    pub fn initial(local_shell_id: Option<String>) -> Self {
        Self {
            id: DEFAULT_CONFIG_ID.to_string(),
            local_shell_id,
            suggest_enabled: true,
            suggest_count: SUGGEST_COUNT_DEFAULT,
            vision_use_cloud: true,
            image_gen_use_cloud: true,
            embedding_use_cloud: true,
            assist_use_cloud: true,
            open_here_enabled: true,
            system_language: SystemLanguage::EnUs as i16,
            ..Self::default()
        }
    }

    pub fn apply_start_defaults(&mut self, default_shell_id: Option<String>, shell_valid: bool) -> bool {
        let mut changed = false;
        if !shell_valid {
            if let Some(shell_id) = default_shell_id {
                self.local_shell_id = Some(shell_id);
                changed = true;
            }
            if !self.suggest_enabled {
                self.suggest_enabled = true;
                changed = true;
            }
        }
        if self.suggest_count < SUGGEST_COUNT_MIN || self.suggest_count > SUGGEST_COUNT_MAX {
            self.suggest_count = SUGGEST_COUNT_DEFAULT;
            changed = true;
        }
        changed
    }
}

pub fn split_config_list(custom: Option<&str>, default: &str) -> Vec<String> {
    let source = custom.map(str::trim).filter(|text| !text.is_empty()).unwrap_or(default);
    source.split(|ch: char| ch.is_whitespace() || matches!(ch, ',' | ';')).filter(|item| !item.is_empty()).map(|item| item.to_ascii_lowercase()).collect()
}

pub type Config = Model;
