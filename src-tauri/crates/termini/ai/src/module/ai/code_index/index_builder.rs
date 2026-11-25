use std::fs;
use std::sync::Arc;
use dashmap::DashMap;
use summer::tracing::warn;
use summer::plugin::Service;
use async_trait::async_trait;
use termini_macros::base_app;
use std::path::{Path, PathBuf};
use crate::base::base_app::BaseApp;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex as AsyncMutex;
use crate::util::time_util::now_millis;
use crate::model::ai_provide::AiProvide;
use std::collections::{BTreeMap, HashMap};
use anyhow::{anyhow, bail, Context, Result};
use crate::service::service_base::BaseService;
use crate::model::resp::resp_model::ResultCode;
use crate::service::service_config::ConfigService;
use crate::service::service_ai_provide::AiProvideService;
use crate::module::ai::code_index::index_guard::normalize_index_dirs;
use crate::module::ai::code_index::{embed_client, semantic, walk_index_files};
use crate::module::ai::code_index::index_scope::{path_in_prefix, resolve_index_scope, IndexScope};

pub const MAX_BATCH_SIZE: usize = 256;
pub const DEFAULT_BATCH_SIZE: usize = 64;
const DATA_DIR_ENV: &str = "TERMINI_DATA_DIR";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "stage", rename_all = "camelCase")]
pub enum IndexStage {
    Scan { files: usize },
    Chunk { total: usize, reuse: usize, pending: usize },
    Embed { done: usize, total: usize },
    Save { chunks: usize },
}

pub type StageSink<'a> = &'a (dyn Fn(IndexStage) + Send + Sync);

#[derive(Clone)]
pub struct EmbedTarget {
    pub provide: AiProvide,
    pub dim: usize,
    pub batch_size: usize,
}

struct IndexPlan {
    pending_text: Vec<String>,
    pending_hash: Vec<String>,
    rows: Vec<semantic::ChunkRow>,
    vectors: Vec<Option<Vec<i8>>>,
    pending_slots: Vec<Vec<usize>>,
    manifest: semantic::IndexManifest,
}

fn create_locks() -> Arc<DashMap<String, Arc<AsyncMutex<()>>>> {
    Arc::new(DashMap::new())
}

#[base_app(sort = 759)]
#[derive(Clone, Service)]
pub struct CodeIndexBuilder {
    #[inject(component)]
    config_service: ConfigService,

    #[inject(component)]
    ai_provide_service: AiProvideService,

    #[inject(func = create_locks())]
    locks: Arc<DashMap<String, Arc<AsyncMutex<()>>>>,
}

#[async_trait]
impl BaseApp for CodeIndexBuilder {
    fn name(&self) -> &'static str {
        "CodeIndexBuilder"
    }
}

impl CodeIndexBuilder {
    pub fn index_dir(root: &Path) -> Result<PathBuf> {
        let data_dir = std::env::var(DATA_DIR_ENV).with_context(|| format!("failed to read {}", DATA_DIR_ENV))?;
        Ok(semantic::index_dir(&PathBuf::from(data_dir), root))
    }

    pub async fn resolve_target(&self) -> Result<EmbedTarget> {
        if let Some(target) = self.try_resolve_target().await? {
            return Ok(target);
        }
        self.config_service.refresh_system_config().await.context("failed to refresh system config")?;
        if let Err(error) = self.ai_provide_service.refresh_cloud().await {
            warn!("[ai] 刷新云端模型列表失败: {error:#}");
        }
        let Some(target) = self.try_resolve_target().await? else {
            bail!("server has no embedding model configured for code index");
        };
        Ok(target)
    }

    async fn try_resolve_target(&self) -> Result<Option<EmbedTarget>> {
        let Some(config) = self.config_service.select_config().await else {
            return Err(ResultCode::DefaultConfigNotFound.into());
        };
        if !config.embedding_use_cloud && config.resolve_embedding_provide_id().is_none() {
            bail!("code index is set to a local embedding model in settings, but no model is selected");
        }
        if !config.embedding_use_cloud && config.resolve_embedding_dim().is_none() {
            bail!("code index is set to a local embedding model in settings, but no vector dimension is set");
        }
        let Some(provide_id) = config.resolve_embedding_provide_id() else {
            return Ok(None);
        };
        let Some(dim) = config.resolve_embedding_dim() else {
            return Ok(None);
        };
        let batch_size = config.embedding_batch_size.filter(|value| *value > 0).map(|value| (value as usize).min(MAX_BATCH_SIZE)).unwrap_or(DEFAULT_BATCH_SIZE);
        let Some(shared) = self.ai_provide_service.find_by_id(provide_id).await? else {
            return Ok(None);
        };
        let provide = shared.read().clone();
        Ok(Some(EmbedTarget { provide, dim: dim as usize, batch_size }))
    }

    pub async fn ignore_rules(&self) -> (Vec<String>, bool) {
        let Some(config) = self.config_service.select_config().await else {
            return (Vec::new(), false);
        };
        let rules = config.index_ignore.clone().unwrap_or_default().lines().map(|line| line.trim().to_string()).filter(|line| !line.is_empty()).collect();
        (rules, config.index_include_tests)
    }

    pub async fn resolve_scope(&self, request: &Path) -> IndexScope {
        let dirs = self.config_service.select_index_dirs().await;
        let (valid, _) = normalize_index_dirs(&dirs);
        resolve_index_scope(request, &valid)
    }

    pub async fn ensure_index(&self, root: &Path, force: bool, on_stage: StageSink<'_>) -> Result<usize> {
        let dir = Self::index_dir(root)?;
        fs::create_dir_all(&dir).with_context(|| format!("failed to create index dir: {}", dir.display()))?;
        let target = self.resolve_target().await?;

        let key = root.to_string_lossy().replace('\\', "/").to_lowercase();
        let lock = self.locks.entry(key).or_insert_with(|| Arc::new(AsyncMutex::new(()))).clone();
        let _guard = lock.lock().await;

        self.build_locked(root, &dir, &target, force, on_stage).await
    }

    pub async fn is_fresh(&self, root: &Path) -> Result<bool> {
        let dir = Self::index_dir(root)?;
        let target = self.resolve_target().await?;
        let (rules, include_tests) = self.ignore_rules().await;
        let root_owned = root.to_path_buf();
        let provide_id = target.provide.id.clone();
        let dim = target.dim;
        tokio::task::spawn_blocking(move || {
            let files = walk_index_files(&root_owned, &rules, include_tests)?;
            let manifest = semantic::current_manifest(&root_owned, &files);
            Ok(is_manifest_fresh(&dir, &provide_id, dim, &manifest))
        })
        .await
        .context("failed to schedule index freshness check")?
    }

    async fn build_locked(&self, root: &Path, dir: &Path, target: &EmbedTarget, force: bool, on_stage: StageSink<'_>) -> Result<usize> {
        let (rules, include_tests) = self.ignore_rules().await;
        let provide_id = target.provide.id.clone();
        let dim = target.dim;

        let plan = {
            let root = root.to_path_buf();
            let dir = dir.to_path_buf();
            let provide_id = provide_id.clone();
            let stages = collect_stages();
            let sink = stages.clone();
            let plan =
                tokio::task::spawn_blocking(move || plan_index(&root, &dir, &rules, include_tests, &provide_id, dim, force, &|stage| sink.lock().push(stage))).await.context("failed to schedule index task")??;
            for stage in stages.lock().drain(..) {
                on_stage(stage);
            }
            plan
        };
        let Some(plan) = plan else {
            return Ok(0);
        };

        let pending = plan.pending_text.len();
        let embedded = if pending == 0 {
            Vec::new()
        } else {
            let report = |done: usize, total: usize| on_stage(IndexStage::Embed { done, total });
            let cache_dir = dir.to_path_buf();
            let pending_hash = &plan.pending_hash;
            let checkpoint = move |start: usize, vectors: &[Vec<f32>]| -> Result<()> {
                let mut entries = Vec::with_capacity(vectors.len());
                for (offset, vector) in vectors.iter().enumerate() {
                    let Some(hash) = pending_hash.get(start + offset) else {
                        continue;
                    };
                    entries.push((hash.clone(), semantic::quantize(vector)));
                }
                semantic::append_vector_cache(&cache_dir, &entries)
            };
            embed_client::embed_all_with_sink(&target.provide, &plan.pending_text, target.batch_size, &report, &checkpoint).await.context("embedding failed")?
        };
        if embedded.len() != pending {
            bail!("vector count mismatch: pending {} returned {}", pending, embedded.len());
        }

        let root_owned = root.to_path_buf();
        let dir_owned = dir.to_path_buf();
        let stages = collect_stages();
        let sink = stages.clone();
        tokio::task::spawn_blocking(move || write_index(&root_owned, &dir_owned, &provide_id, dim, plan, embedded, &|stage| sink.lock().push(stage))).await.context("failed to schedule index write")??;
        for stage in stages.lock().drain(..) {
            on_stage(stage);
        }
        Ok(pending)
    }
}

type StageBuffer = Arc<parking_lot::Mutex<Vec<IndexStage>>>;

fn collect_stages() -> StageBuffer {
    Arc::new(parking_lot::Mutex::new(Vec::new()))
}

fn is_manifest_fresh(dir: &Path, provide_id: &str, dim: usize, manifest: &semantic::IndexManifest) -> bool {
    semantic::load_meta(dir)
        .filter(|meta| semantic::meta_matches(meta, provide_id, dim))
        .and_then(|meta| semantic::load_manifest(dir).map(|stored| (meta, stored)))
        .map(|(meta, stored)| stored.files == manifest.files && meta.chunk_count > 0)
        .unwrap_or(false)
}

fn plan_index(root: &Path, dir: &Path, rules: &[String], include_tests: bool, provide_id: &str, dim: usize, force: bool, on_stage: &dyn Fn(IndexStage)) -> Result<Option<IndexPlan>> {
    let files = walk_index_files(root, rules, include_tests)?;
    let manifest = semantic::current_manifest(root, &files);
    if is_manifest_fresh(dir, provide_id, dim, &manifest) && !force {
        return Ok(None);
    }
    on_stage(IndexStage::Scan { files: files.len() });
    let inputs = semantic::build_chunks(root, &files);
    if inputs.is_empty() {
        bail!("no indexable files: {}", root.display());
    }
    let mut reuse = 0usize;
    let mut pending_text = Vec::new();
    let mut pending_hash = Vec::new();
    let mut rows = Vec::with_capacity(inputs.len());
    let cache = load_cache(dir, provide_id, dim, force);
    let mut pending_slots: Vec<Vec<usize>> = Vec::new();
    let mut pending_index: HashMap<String, usize> = HashMap::new();
    let mut vectors: Vec<Option<Vec<i8>>> = Vec::with_capacity(inputs.len());
    for (slot, input) in inputs.into_iter().enumerate() {
        match cache.get(&input.row.hash) {
            Some(existing) => {
                vectors.push(Some(existing.clone()));
                reuse += 1;
            }
            None => {
                vectors.push(None);
                match pending_index.get(&input.row.hash) {
                    Some(position) => pending_slots[*position].push(slot),
                    None => {
                        pending_index.insert(input.row.hash.clone(), pending_text.len());
                        pending_hash.push(input.row.hash.clone());
                        pending_text.push(input.text);
                        pending_slots.push(vec![slot]);
                    }
                }
            }
        }
        rows.push(input.row);
    }
    on_stage(IndexStage::Chunk { total: rows.len(), reuse, pending: pending_text.len() });
    Ok(Some(IndexPlan { manifest, rows, vectors, pending_text, pending_hash, pending_slots }))
}

fn write_index(root: &Path, dir: &Path, provide_id: &str, dim: usize, plan: IndexPlan, embedded: Vec<Vec<f32>>, on_stage: &dyn Fn(IndexStage)) -> Result<()> {
    let IndexPlan { manifest, rows, mut vectors, pending_slots, .. } = plan;
    for (position, vector) in embedded.into_iter().enumerate() {
        if vector.len() != dim {
            bail!("vector dimension mismatch: configured {} actual {}", dim, vector.len());
        }
        let Some(slots) = pending_slots.get(position) else {
            bail!("vector count exceeds pending chunk count");
        };
        let quantized = semantic::quantize(&vector);
        for slot in slots {
            vectors[*slot] = Some(quantized.clone());
        }
    }
    let mut flat = Vec::with_capacity(rows.len() * dim);
    for slot in vectors.iter_mut() {
        let Some(vector) = slot.take() else {
            bail!("missing vector");
        };
        flat.extend(vector);
    }
    let meta = semantic::build_meta(root, provide_id, dim, rows.len(), now_millis());
    semantic::save_index(dir, &meta, &manifest, &rows, &flat).context("failed to write index")?;
    on_stage(IndexStage::Save { chunks: rows.len() });
    Ok(())
}

fn load_cache(dir: &Path, provide_id: &str, dim: usize, force: bool) -> BTreeMap<String, Vec<i8>> {
    if force || !semantic::cache_meta_matches(dir, provide_id, dim) {
        if let Err(error) = semantic::reset_vector_cache(dir, provide_id, dim) {
            warn!("[ai] 重置向量缓存失败: {error:#}");
        }
        return BTreeMap::new();
    }
    let mut cache = semantic::load_vector_cache(dir, provide_id, dim);
    if !cache.is_empty() {
        return cache;
    }
    let Some(meta) = semantic::load_meta(dir) else {
        return cache;
    };
    if meta.dim != dim || meta.provide_id != provide_id {
        return cache;
    }
    let Ok(rows) = semantic::load_rows(dir) else {
        return cache;
    };
    let Ok(vectors) = semantic::load_vectors(dir, dim, rows.len()) else {
        return cache;
    };
    let mut entries = Vec::with_capacity(rows.len());
    for (index, row) in rows.into_iter().enumerate() {
        let offset = index * dim;
        let vector = vectors[offset..offset + dim].to_vec();
        cache.insert(row.hash.clone(), vector.clone());
        entries.push((row.hash, vector));
    }
    if let Err(error) = semantic::append_vector_cache(dir, &entries) {
        warn!("[ai] 回填向量缓存失败: {error:#}");
    }
    cache
}

pub fn search_index(dir: &Path, dim: usize, query: &[f32], top_k: usize) -> Result<(Vec<semantic::ChunkRow>, Vec<(usize, f32)>)> {
    search_index_scoped(dir, dim, query, top_k, None)
}

pub fn search_index_scoped(dir: &Path, dim: usize, query: &[f32], top_k: usize, prefix: Option<&String>) -> Result<(Vec<semantic::ChunkRow>, Vec<(usize, f32)>)> {
    let rows = semantic::load_rows(dir).context("failed to read index")?;
    let vectors = semantic::load_vectors(dir, dim, rows.len()).context("failed to read vectors")?;
    let allow: Option<Vec<bool>> = prefix.map(|value| rows.iter().map(|row| path_in_prefix(&row.path, Some(value))).collect());
    let hits = semantic::search_filtered(query, &vectors, dim, top_k, allow.as_deref());
    Ok((rows, hits))
}

pub fn index_summary(dir: &Path) -> Option<(usize, i64)> {
    let meta = semantic::load_meta(dir)?;
    Some((meta.chunk_count, meta.built_at))
}

pub fn ensure_absolute_dir(path: &str) -> Result<PathBuf> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        bail!("directory is empty");
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        bail!("must be an absolute path: {}", trimmed);
    }
    if !path.exists() {
        bail!("directory does not exist: {}", path.display());
    }
    if !path.is_dir() {
        return Err(anyhow!("not a directory: {}", path.display()));
    }
    Ok(path)
}
