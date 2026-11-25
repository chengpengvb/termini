use rand::RngCore;
use std::sync::Arc;
use serde_json::Value;
use summer::plugin::Service;
use std::collections::HashMap;
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use summer::tracing::{info, warn};
use serde::{Deserialize, Serialize};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use argon2::{Algorithm, Argon2, Version};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use anyhow::{anyhow, bail, Context, Result};
use crate::model::resp::resp_model::ResultCode;
use crate::model::cloud_sync_state::CloudSyncState;
use crate::service::service_base_api::BaseApiService;
use crate::dao::dao_cloud_sync_state::CloudSyncStateDao;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use crate::service::service_account_token::AccountTokenService;
use crate::model::resp::resp_user_cloud_sync::RespUserCloudSync;
use crate::service::service_base_sync::{self, OP_DOWNLOAD, OP_UPLOAD};
use crate::model::req::req_cloud_sync::{CloudSyncChoice, ReqUserCloudSyncItem};
use crate::model::resp::resp_cloud_sync::{RespCloudSyncConflict, RespCloudSyncPrepare, RespCloudSyncStatus};

const SIDE_LOCAL: i8 = 1;
const SIDE_CLOUD: i8 = 2;

fn create_busy() -> Arc<tokio::sync::Mutex<()>> {
    Arc::new(tokio::sync::Mutex::new(()))
}

#[derive(Serialize, Deserialize)]
struct VerifyPayload {
    magic: String,
    version: String,
}

#[derive(Clone, Service)]
pub struct CloudSyncService {
    #[inject(component)]
    account_token_service: AccountTokenService,

    #[inject(component)]
    cloud_sync_state_dao: CloudSyncStateDao,

    #[inject(func = create_busy())]
    busy: Arc<tokio::sync::Mutex<()>>,
}

impl CloudSyncService {
    const SYNC_TYPE_VERIFY: i8 = 0;
    const PASSWORD_MIN_LEN: usize = 6;
    const PASSWORD_MAX_LEN: usize = 20;
    const VERIFY_MAGIC: &'static [u8] = b"termini.cloud.sync.v1";
    const VERIFY_MAGIC_STR: &'static str = "termini.cloud.sync.v1";

    pub async fn prepare(&self, op: i8, password: String) -> Result<RespCloudSyncPrepare> {
        Self::require_password(&password)?;
        if op != OP_UPLOAD && op != OP_DOWNLOAD {
            return Err(ResultCode::CloudSyncOpInvalid.into());
        }
        info!("[sync] cloud sync prepare op={}", op);
        self.require_token().await?;
        let cloud_all = self.download_all().await?;
        let cloud_version = match cloud_all.get(&Self::SYNC_TYPE_VERIFY) {
            None => {
                if op == OP_DOWNLOAD {
                    return Err(ResultCode::CloudSyncNotFound.into());
                }
                return Ok(RespCloudSyncPrepare { conflicts: Vec::new() });
            }
            Some(payload) => decode_verify(&password, payload)?,
        };
        let base_version = self.load_base_version().await?;
        if let (Some(cloud_version), Some(base_version)) = (cloud_version.as_ref(), base_version.as_ref()) {
            if cloud_version == base_version {
                info!("[sync] cloud sync prepare op={} cloud unchanged since last sync", op);
                return Ok(RespCloudSyncPrepare { conflicts: Vec::new() });
            }
        }
        let mut conflicts = Vec::new();
        let mut cached: Option<([u8; 16], [u8; 64])> = None;
        for syncer in service_base_sync::all() {
            let sync_type = syncer.sync_type();
            let Some(payload) = cloud_all.get(&sync_type) else {
                continue;
            };
            let cloud = decode_list(&password, sync_type, payload, &mut cached)?;
            let mut local_map: HashMap<String, Value> = HashMap::new();
            for item in expect_objects(syncer.export_current().await?)? {
                local_map.insert(object_id(&item)?, item);
            }
            let skip = syncer.skip_id();
            for cloud_item in cloud {
                let id = object_id(&cloud_item)?;
                if Some(id.as_str()) == skip {
                    continue;
                }
                let Some(local_item) = local_map.get(&id) else {
                    continue;
                };
                if local_item != &cloud_item {
                    conflicts.push(RespCloudSyncConflict { sync_type, id, local: local_item.clone(), cloud: cloud_item });
                }
            }
        }
        info!("[sync] cloud sync prepare op={} conflicts={}", op, conflicts.len());
        Ok(RespCloudSyncPrepare { conflicts })
    }

    pub async fn apply(&self, op: i8, password: String, choices: Vec<CloudSyncChoice>) -> Result<()> {
        let Ok(_guard) = self.busy.try_lock() else {
            warn!("[sync] cloud sync apply rejected: busy");
            return Err(ResultCode::CloudSyncBusy.into());
        };
        Self::require_password(&password)?;
        if op != OP_UPLOAD && op != OP_DOWNLOAD {
            return Err(ResultCode::CloudSyncOpInvalid.into());
        }
        info!("[sync] cloud sync apply op={}", op);
        self.require_token().await?;
        let cloud_all = self.download_all().await?;
        let mut upload_items: Vec<ReqUserCloudSyncItem> = Vec::new();
        let salt = random_bytes::<16>();
        let key = derive_key(&password, &salt)?;
        let new_version = uuid();
        let cloud_version = match cloud_all.get(&Self::SYNC_TYPE_VERIFY) {
            None => {
                if op == OP_DOWNLOAD {
                    return Err(ResultCode::CloudSyncNotFound.into());
                }
                info!("[sync] cloud sync apply first time verify");
                None
            }
            Some(payload) => decode_verify(&password, payload)?,
        };
        let mut choice_map: HashMap<(i8, String), i8> = HashMap::new();
        for choice in choices {
            if choice.side != SIDE_LOCAL && choice.side != SIDE_CLOUD {
                return Err(ResultCode::CloudSyncChoiceInvalid.into());
            }
            choice_map.insert((choice.sync_type, choice.id), choice.side);
        }
        let base_version = self.load_base_version().await?;
        let cloud_unchanged = matches!((&cloud_version, &base_version), (Some(cloud_version), Some(base_version)) if cloud_version == base_version);
        let mut cached: Option<([u8; 16], [u8; 64])> = None;
        let syncers = service_base_sync::all();
        let mut merged_list: Vec<Value> = Vec::with_capacity(syncers.len());
        for syncer in syncers {
            let sync_type = syncer.sync_type();
            let cloud = match cloud_all.get(&sync_type) {
                Some(payload) => decode_list(&password, sync_type, payload, &mut cached)?,
                None => Vec::new(),
            };
            let local = expect_objects(syncer.export_current().await?)?;
            let merged = build_merged(sync_type, syncer.skip_id(), &cloud, &local, &choice_map, cloud_unchanged)?;
            let merged = syncer.merged(merged, &cloud, &local, op)?;
            let merged_value = Value::Array(merged);
            if op == OP_UPLOAD {
                let plain = serde_json::to_vec(&merged_value)?;
                let payload = encrypt(&key, &salt, sync_type, &plain)?;
                upload_items.push(ReqUserCloudSyncItem { r#type: sync_type, payload });
            }
            merged_list.push(merged_value);
        }
        if op == OP_UPLOAD {
            let payload = encode_verify(&key, &salt, &new_version)?;
            upload_items.push(ReqUserCloudSyncItem { r#type: Self::SYNC_TYPE_VERIFY, payload });
            self.upload_all(upload_items).await?;
            self.save_base_version(&new_version).await?;
        } else if let Some(cloud_version) = cloud_version {
            self.save_base_version(&cloud_version).await?;
        }
        for (syncer, merged_value) in syncers.iter().zip(merged_list) {
            let sync_type = syncer.sync_type();
            syncer.import_cloud(merged_value).await?;
            info!("[sync] cloud sync apply op={} type={} ok", op, sync_type);
        }
        info!("[sync] cloud sync apply op={} done", op);
        Ok(())
    }

    fn require_password(password: &str) -> Result<()> {
        if password.chars().count() < Self::PASSWORD_MIN_LEN {
            return Err(ResultCode::PasswordTooShort.into());
        }
        if password.chars().count() > Self::PASSWORD_MAX_LEN {
            return Err(ResultCode::PasswordTooLong.into());
        }
        Ok(())
    }

    async fn load_base_version(&self) -> Result<Option<String>> {
        match self.cloud_sync_state_dao.find_by_id(CloudSyncState::DEFAULT_ID.to_string()).await? {
            Some(shared) => {
                let version = shared.read().version.clone();
                Ok(if version.is_empty() { None } else { Some(version) })
            }
            None => Ok(None),
        }
    }

    async fn save_base_version(&self, version: &str) -> Result<()> {
        let model = CloudSyncState { id: CloudSyncState::DEFAULT_ID.to_string(), version: version.to_string() };
        match self.cloud_sync_state_dao.find_by_id(CloudSyncState::DEFAULT_ID.to_string()).await? {
            Some(_) => {
                self.cloud_sync_state_dao.update(model).await?;
            }
            None => {
                self.cloud_sync_state_dao.insert(model).await?;
            }
        }
        Ok(())
    }

    async fn require_token(&self) -> Result<()> {
        if self.account_token_service.get_token().await?.is_none() {
            return Err(ResultCode::Unauthorized.into());
        }
        Ok(())
    }

    pub async fn status(&self) -> Result<RespCloudSyncStatus> {
        self.require_token().await?;
        Self::api_data(Self::api_post("/api/userCloudSyncApi/status")).await
    }

    async fn download_all(&self) -> Result<HashMap<i8, String>> {
        let rows: Vec<RespUserCloudSync> = Self::api_data(Self::api_post("/api/userCloudSyncApi/downloadAll")).await?;
        let mut map: HashMap<i8, String> = HashMap::new();
        for row in rows {
            map.insert(row.r#type, row.payload);
        }
        Ok(map)
    }

    async fn upload_all(&self, items: Vec<ReqUserCloudSyncItem>) -> Result<()> {
        Self::api_result::<Value>(Self::api_post("/api/userCloudSyncApi/uploadAll").json(&items)).await?;
        Ok(())
    }
}

impl BaseApiService for CloudSyncService {}

fn decode_verify(password: &str, payload: &str) -> Result<Option<String>> {
    let (salt, nonce, ct) = parse_envelope(payload)?;
    let key = derive_key(password, &salt)?;
    let plain = decrypt(&key, CloudSyncService::SYNC_TYPE_VERIFY, &nonce, &ct).map_err(|_| anyhow::Error::from(ResultCode::PasswordMismatch))?;
    if let Ok(verify) = serde_json::from_slice::<VerifyPayload>(&plain) {
        if verify.magic != CloudSyncService::VERIFY_MAGIC_STR {
            return Err(ResultCode::PasswordMismatch.into());
        }
        return Ok(Some(verify.version));
    }
    if plain == CloudSyncService::VERIFY_MAGIC {
        return Ok(None);
    }
    Err(ResultCode::PasswordMismatch.into())
}

fn encode_verify(key: &[u8; 64], salt: &[u8; 16], version: &str) -> Result<String> {
    let plain = serde_json::to_vec(&VerifyPayload { magic: CloudSyncService::VERIFY_MAGIC_STR.to_string(), version: version.to_string() })?;
    encrypt(key, salt, CloudSyncService::SYNC_TYPE_VERIFY, &plain)
}

fn decode_list(password: &str, sync_type: i8, payload: &str, cached: &mut Option<([u8; 16], [u8; 64])>) -> Result<Vec<Value>> {
    let (salt, nonce, ct) = parse_envelope(payload)?;
    let key = match &cached {
        Some((s, k)) if s == &salt => *k,
        _ => {
            let k = derive_key(password, &salt)?;
            *cached = Some((salt, k));
            k
        }
    };
    let plain = decrypt(&key, sync_type, &nonce, &ct)?;
    let value = serde_json::from_slice(&plain)?;
    expect_objects(value)
}

fn random_bytes<const N: usize>() -> [u8; N] {
    let mut buf = [0u8; N];
    rand::thread_rng().fill_bytes(&mut buf);
    buf
}

fn derive_key(password: &str, salt: &[u8; 16]) -> Result<[u8; 64]> {
    let params = argon2::Params::new(65536, 3, 1, Some(64)).map_err(|e| anyhow!(e))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 64];
    argon.hash_password_into(password.as_bytes(), salt, &mut key).map_err(|e| anyhow!(e))?;
    Ok(key)
}

fn encrypt(key: &[u8; 64], salt: &[u8; 16], sync_type: i8, plain: &[u8]) -> Result<String> {
    let nonce = random_bytes::<12>();
    let aad = [1u8, sync_type as u8];
    let cipher = <Aes256Gcm as KeyInit>::new(Key::<Aes256Gcm>::from_slice(&key[..32]));
    let ct = cipher.encrypt(Nonce::from_slice(&nonce), Payload { msg: plain, aad: &aad }).map_err(|_| anyhow!("encrypt failed"))?;
    let mut out = Vec::with_capacity(1 + 16 + 12 + ct.len());
    out.push(1);
    out.extend_from_slice(salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ct);
    Ok(STANDARD.encode(out))
}

fn parse_envelope(payload: &str) -> Result<([u8; 16], [u8; 12], Vec<u8>)> {
    let raw = STANDARD.decode(payload)?;
    if raw.len() < 1 + 16 + 12 + 16 {
        bail!("invalid cloud sync payload");
    }
    if raw[0] != 1 {
        bail!("unsupported cloud sync payload version");
    }
    let mut salt = [0u8; 16];
    salt.copy_from_slice(&raw[1..17]);
    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&raw[17..29]);
    Ok((salt, nonce, raw[29..].to_vec()))
}

fn decrypt(key: &[u8; 64], sync_type: i8, nonce: &[u8; 12], ct: &[u8]) -> Result<Vec<u8>> {
    let cipher = <Aes256Gcm as KeyInit>::new(Key::<Aes256Gcm>::from_slice(&key[..32]));
    let aad = [1u8, sync_type as u8];
    cipher.decrypt(Nonce::from_slice(nonce), Payload { msg: ct, aad: &aad }).map_err(|_| anyhow!("decrypt failed"))
}

fn build_merged(sync_type: i8, skip_id: Option<&str>, cloud: &[Value], local: &[Value], choices: &HashMap<(i8, String), i8>, cloud_unchanged: bool) -> Result<Vec<Value>> {
    if cloud_unchanged {
        let mut result = Vec::with_capacity(local.len());
        for item in local {
            let id = object_id(item)?;
            if Some(id.as_str()) == skip_id {
                continue;
            }
            result.push(item.clone());
        }
        return Ok(result);
    }
    let mut merged: HashMap<String, Value> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for item in cloud {
        let id = object_id(item)?;
        if Some(id.as_str()) == skip_id {
            continue;
        }
        if !merged.contains_key(&id) {
            order.push(id.clone());
        }
        merged.insert(id, item.clone());
    }
    for item in local {
        let id = object_id(item)?;
        if Some(id.as_str()) == skip_id {
            continue;
        }
        match merged.get(&id) {
            None => {
                order.push(id.clone());
                merged.insert(id, item.clone());
            }
            Some(cloud_item) => {
                if cloud_item == item {
                    continue;
                }
                let Some(side) = choices.get(&(sync_type, id.clone())) else {
                    return Err(ResultCode::CloudSyncChoiceInvalid.into());
                };
                if *side == SIDE_LOCAL {
                    merged.insert(id, item.clone());
                }
            }
        }
    }
    let mut result = Vec::with_capacity(order.len());
    for id in order {
        let item = merged.remove(&id).context("cloud sync merged item missing")?;
        result.push(item);
    }
    Ok(result)
}

fn expect_objects(data: Value) -> Result<Vec<Value>> {
    let Value::Array(items) = data else {
        bail!("cloud sync payload must be array");
    };
    for item in &items {
        if !item.is_object() {
            bail!("cloud sync item must be object");
        }
    }
    Ok(items)
}

fn object_id(item: &Value) -> Result<String> {
    object_string(item, "id")
}

fn object_string(item: &Value, key: &str) -> Result<String> {
    let value = item.get(key).with_context(|| format!("cloud sync item missing {}", key))?;
    let Value::String(value) = value else {
        bail!("cloud sync item {} must be string", key);
    };
    if value.is_empty() {
        bail!("cloud sync item {} is empty", key);
    }
    Ok(value.clone())
}
