use std::sync::OnceLock;
use crate::model::config;
use anyhow::{bail, Result};
use summer::plugin::Service;
use async_trait::async_trait;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use tokio::sync::{broadcast, Mutex};
use crate::dao::dao_config::ConfigDao;
use crate::util::time_util::now_millis;
use summer::tracing::{error, info, warn};
use termini_macros::{base_app, base_service};
use crate::model::resp::resp_model::ResultCode;
use crate::base::base_app::{AppContext, BaseApp};
use crate::model::resp::resp_login_url::RespLoginUrl;
use crate::model::server::system_config::SystemConfig;
use crate::service::service_base_api::{ApiEvent, BaseApiService};
use crate::model::req::req_config::{ReqSaveNotify, ReqSaveRoleModel};
use crate::module::shell::platform::shell_discover::native_shell_platform;
use crate::model::config::{Config, SystemLanguage, DEFAULT_CONFIG_ID, SUGGEST_COUNT_MAX, SUGGEST_COUNT_MIN};

#[base_app(sort = 200)]
#[base_service(config::Entity, dao = "config_dao")]
#[derive(Clone, Service)]
pub struct ConfigService {
    #[inject(component)]
    config_dao: ConfigDao,
}

#[async_trait]
impl BaseApp for ConfigService {
    fn name(&self) -> &'static str {
        "ConfigService"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        self.ensure_default_config().await;
        if let Err(err) = self.refresh_system_config().await {
            warn!("[config] 同步系统配置失败: {err:#}");
        }
        self.ensure_default_config().await;
        let svc = self.clone();
        tokio::spawn(async move {
            let mut rx = Self::api_subscribe();
            loop {
                match rx.recv().await {
                    Ok(ApiEvent::ConfigVersionChanged { .. }) => svc.on_config_version_changed().await,
                    Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                    Err(broadcast::error::RecvError::Closed) => return,
                }
            }
        });
        Ok(())
    }
}

impl BaseApiService for ConfigService {}

impl ConfigService {
    pub async fn select_config(&self) -> Option<Config> {
        match self.config_dao.find_by_id(DEFAULT_CONFIG_ID.to_string()).await {
            Ok(config_option) => match config_option {
                None => {
                    error!("[config] default config is not found");
                    None
                }
                Some(config) => Some(config.read().clone()),
            },
            Err(e) => {
                error!("[config] Failed to find default config: {}", e);
                None
            }
        }
    }

    pub async fn select_system_language(&self) -> SystemLanguage {
        match self.select_config().await {
            Some(config) => config.resolve_system_language(),
            None => Self::resolve_os_language(),
        }
    }

    fn resolve_os_language() -> SystemLanguage {
        match sys_locale::get_locale() {
            Some(tag) => SystemLanguage::from_locale(&tag),
            None => SystemLanguage::EnUs,
        }
    }

    pub async fn fetch_system_config(&self) -> Result<SystemConfig> {
        Self::api_data(Self::api_get("/api/systemConfigApi/config")).await
    }

    fn config_changed_sender() -> &'static broadcast::Sender<()> {
        static SENDER: OnceLock<broadcast::Sender<()>> = OnceLock::new();
        SENDER.get_or_init(|| broadcast::channel(4).0)
    }

    pub fn subscribe_config_changed() -> broadcast::Receiver<()> {
        Self::config_changed_sender().subscribe()
    }

    fn write_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    async fn mutate(&self, apply: impl FnOnce(&mut Config) + Send) -> Result<()> {
        let _guard = Self::write_lock().lock().await;
        let Some(mut config) = self.select_config().await else {
            return Err(ResultCode::DefaultConfigNotFound.into());
        };
        apply(&mut config);
        self.config_dao.update(config).await?;
        Ok(())
    }

    async fn on_config_version_changed(&self) {
        if let Err(err) = self.refresh_system_config().await {
            warn!("[config] 配置版本变化后刷新系统配置失败: {err:#}");
        }
        self.ensure_default_config().await;
        let _ = Self::config_changed_sender().send(());
    }

    pub async fn refresh_system_config(&self) -> Result<()> {
        let data = self.fetch_system_config().await?;
        let _guard = Self::write_lock().lock().await;
        let Some(shared) = self.config_dao.find_by_id(DEFAULT_CONFIG_ID.to_string()).await? else {
            return Err(ResultCode::DefaultConfigNotFound.into());
        };
        let mut config = shared.write();

        config.embedding_dim = data.embedding_dim;
        config.vision_max_pixels = data.vision_max_pixels;
        config.login_url = Self::trim_option(data.login_url);
        config.vision_jpeg_quality = data.vision_jpeg_quality;
        config.portal_url = Self::trim_option(data.portal_url);
        config.embedding_batch_size = data.embedding_batch_size;
        config.suggest_free_monthly = data.suggest_free_monthly;
        config.tool_mirror_url = Self::trim_option(data.tool_mirror_url);
        config.assist_ai_provide_id = Self::trim_option(data.assist_ai_provide_id);
        config.vision_ai_provide_id = Self::trim_option(data.vision_ai_provide_id);
        config.suggest_ai_provide_id = Self::trim_option(data.suggest_ai_provide_id);
        config.image_gen_ai_provide_id = Self::trim_option(data.image_gen_ai_provide_id);
        config.embedding_ai_provide_id = Self::trim_option(data.embedding_ai_provide_id);
        Ok(())
    }

    pub async fn login_url(&self) -> Result<RespLoginUrl> {
        if let Err(err) = self.refresh_system_config().await {
            warn!("[config] 刷新登录地址失败, 回退本地缓存: {err:#}");
        }
        let Some(config) = self.select_config().await else {
            return Err(ResultCode::DefaultConfigNotFound.into());
        };
        let Some(login_url) = config.login_url.filter(|item| !item.trim().is_empty()) else {
            return Err(ResultCode::LoginUrlEmpty.into());
        };
        let url = if login_url.contains('?') { format!("{}&termini=1", login_url) } else { format!("{}?termini=1", login_url) };
        Ok(RespLoginUrl { url })
    }

    pub async fn portal_url(&self) -> Result<RespLoginUrl> {
        if let Err(err) = self.refresh_system_config().await {
            warn!("[config] 刷新账号中心地址失败, 回退本地缓存: {err:#}");
        }
        let Some(config) = self.select_config().await else {
            return Err(ResultCode::DefaultConfigNotFound.into());
        };
        let Some(portal_url) = config.portal_url.filter(|item| !item.trim().is_empty()) else {
            return Err(ResultCode::LoginUrlEmpty.into());
        };
        Ok(RespLoginUrl { url: portal_url })
    }

    pub async fn save_suggest(&self, enabled: bool, count: i16) -> Result<()> {
        self.mutate(|config| {
            config.suggest_enabled = enabled;
            config.suggest_count = count.clamp(SUGGEST_COUNT_MIN, SUGGEST_COUNT_MAX);
        })
        .await
    }

    pub async fn save_code_index(&self, ignore: Option<String>, include_tests: bool) -> Result<()> {
        self.mutate(|config| {
            config.index_ignore = Self::trim_option(ignore);
            config.index_include_tests = include_tests;
        })
        .await
    }

    pub async fn save_index_dirs(&self, dirs: Vec<String>) -> Result<()> {
        self.mutate(|config| config.index_dirs = if dirs.is_empty() { None } else { Some(dirs.join("\n")) }).await
    }

    pub async fn select_index_dirs(&self) -> Vec<String> {
        let Some(config) = self.select_config().await else {
            return Vec::new();
        };
        config.index_dirs.unwrap_or_default().lines().map(|line| line.trim().to_string()).filter(|line| !line.is_empty()).collect()
    }

    pub async fn ensure_client_device_id(&self) -> Result<String> {
        let _guard = Self::write_lock().lock().await;
        let Some(mut config) = self.select_config().await else {
            return Err(ResultCode::DefaultConfigNotFound.into());
        };
        if let Some(id) = config.client_device_id.clone().filter(|item| !item.trim().is_empty()) {
            return Ok(id);
        }
        let id = uuid();
        config.client_device_id = Some(id.clone());
        self.config_dao.update(config).await?;
        info!("[config] 生成本机设备标识: {}", id);
        Ok(id)
    }

    pub async fn save_system(&self, system_language: i16, local_shell_id: Option<String>, shell_cpu_percent: i16) -> Result<()> {
        self.mutate(|config| {
            config.system_language = SystemLanguage::from_i16(system_language) as i16;
            config.local_shell_id = Self::trim_option(local_shell_id);
            config.shell_cpu_percent = shell_cpu_percent.clamp(config::SHELL_CPU_PERCENT_MIN, config::SHELL_CPU_PERCENT_MAX);
        })
        .await
    }

    pub async fn save_job_collect(&self, exec_timeout_secs: i64, worker_parallelism: i16, indicator_value_cache_size: i16) -> Result<()> {
        self.mutate(|config| {
            config.job_exec_timeout_secs = exec_timeout_secs.clamp(config::JOB_EXEC_TIMEOUT_SECS_MIN, config::JOB_EXEC_TIMEOUT_SECS_MAX);
            config.job_worker_parallelism = worker_parallelism.clamp(config::JOB_WORKER_PARALLELISM_MIN, config::JOB_WORKER_PARALLELISM_MAX);
            config.job_indicator_value_cache_size = indicator_value_cache_size.clamp(config::JOB_INDICATOR_VALUE_CACHE_SIZE_MIN, config::JOB_INDICATOR_VALUE_CACHE_SIZE_MAX);
        })
        .await
    }

    pub async fn save_job_ssh_pool(&self, max_size: i16, idle_secs: i64) -> Result<()> {
        self.mutate(|config| {
            config.job_ssh_pool_max_size = max_size.clamp(config::JOB_SSH_POOL_MAX_SIZE_MIN, config::JOB_SSH_POOL_MAX_SIZE_MAX);
            config.job_ssh_pool_idle_secs = idle_secs.clamp(config::JOB_SSH_POOL_IDLE_SECS_MIN, config::JOB_SSH_POOL_IDLE_SECS_MAX);
        })
        .await
    }

    pub async fn save_audit_policy(&self, audit_policy: Option<String>) -> Result<()> {
        self.mutate(|config| config.audit_policy = Self::trim_option(audit_policy)).await
    }

    pub async fn save_job_alert(&self, ai_provide_id: Option<String>) -> Result<()> {
        self.mutate(|config| config.job_alert_ai_provide_id = Self::trim_option(ai_provide_id)).await
    }

    pub async fn save_job_discover(&self, ai_provide_id: Option<String>, parallelism: i16) -> Result<()> {
        self.mutate(|config| {
            config.job_discover_ai_provide_id = Self::trim_option(ai_provide_id);
            config.job_discover_parallelism = parallelism.clamp(config::JOB_DISCOVER_PARALLELISM_MIN, config::JOB_DISCOVER_PARALLELISM_MAX);
        })
        .await
    }

    pub async fn save_role_model(&self, req: ReqSaveRoleModel) -> Result<()> {
        if !matches!(req.role, config::MODEL_ROLE_VISION | config::MODEL_ROLE_IMAGE_GEN | config::MODEL_ROLE_EMBEDDING | config::MODEL_ROLE_ASSIST) {
            bail!("unknown model role: {}", req.role);
        }
        self.mutate(|config| {
            let local_id = Self::trim_option(req.ai_provide_id);
            match req.role {
                config::MODEL_ROLE_VISION => {
                    config.vision_use_cloud = req.use_cloud;
                    config.vision_local_ai_provide_id = local_id;
                }
                config::MODEL_ROLE_IMAGE_GEN => {
                    config.image_gen_use_cloud = req.use_cloud;
                    config.image_gen_local_ai_provide_id = local_id;
                }
                config::MODEL_ROLE_EMBEDDING => {
                    config.embedding_use_cloud = req.use_cloud;
                    config.embedding_local_ai_provide_id = local_id;
                    config.embedding_local_dim = req.dim.filter(|value| *value > 0);
                }
                _ => {
                    config.assist_use_cloud = req.use_cloud;
                    config.assist_local_ai_provide_id = local_id;
                }
            }
        })
        .await
    }

    pub async fn save_notify(&self, req: ReqSaveNotify) -> Result<()> {
        self.mutate(|config| {
            config.notify_email = Self::trim_option(req.email);
            config.notify_enabled = req.enabled;
            config.notify_tray_enabled = req.tray_enabled;
            config.notify_webhook_enabled = req.webhook_enabled;
            config.notify_webhook_kind = req.webhook_kind;
            config.notify_webhook_url = Self::trim_option(req.webhook_url);
            config.notify_webhook_extra = Self::trim_option(req.webhook_extra);
        })
        .await
    }

    pub async fn save_close_action(&self, close_action: i16) -> Result<()> {
        self.mutate(|config| config.close_action = close_action.clamp(config::CLOSE_ACTION_ASK, config::CLOSE_ACTION_TRAY)).await
    }

    pub async fn save_open_here(&self, enabled: bool) -> Result<()> {
        self.mutate(|config| config.open_here_enabled = enabled).await
    }

    pub async fn touch_job_heartbeat(&self) -> Result<()> {
        self.mutate(|config| config.job_heartbeat_at = now_millis()).await
    }

    fn trim_option(value: Option<String>) -> Option<String> {
        value.map(|item| item.trim().to_string()).filter(|item| !item.is_empty())
    }

    async fn ensure_default_config(&self) {
        let _guard = Self::write_lock().lock().await;
        let found = match self.config_dao.find_by_id(DEFAULT_CONFIG_ID.to_string()).await {
            Ok(config) => config,
            Err(e) => return error!("[config] Failed to find default config: {}", e),
        };
        let shells = native_shell_platform().discover();
        let default_shell_id = shells.first().map(|item| item.shell_id.clone());
        if found.is_none() {
            let mut config = Config::initial(default_shell_id.clone());
            config.system_language = Self::resolve_os_language() as i16;
            if let Err(e) = self.config_dao.insert(config).await {
                error!("[config] Failed to insert default config: {}", e);
                return;
            }
            info!("[config] 创建默认配置[{}].", DEFAULT_CONFIG_ID);
        }
        let Some(mut config) = self.select_config().await else {
            return;
        };
        let shell_valid = config.local_shell_id.as_ref().is_some_and(|id| shells.iter().any(|item| &item.shell_id == id));
        if config.apply_start_defaults(default_shell_id, shell_valid) {
            if let Err(e) = self.config_dao.update(config).await {
                error!("[config] Failed to update default config: {}", e);
            }
        }
    }
}
