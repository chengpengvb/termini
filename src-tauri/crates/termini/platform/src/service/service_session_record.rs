use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use dashmap::DashMap;
use serde_json::json;
use std::path::PathBuf;
use std::time::Instant;
use summer::tracing::warn;
use summer::plugin::Service;
use anyhow::{anyhow, Context};
use crate::dao::dao_base::BaseDao;
use crate::util::id_factory::uuid;
use parking_lot::Mutex as SyncMutex;
use crate::util::time_util::now_millis;
use crate::model::dto::shared_ptr::Shared;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::base::config::app_config::AppConfig;
use crate::model::session_record::SessionRecord;
use crate::dao::dao_session_record::SessionRecordDao;
use crate::module::shell::shell_session_hub::is_terminal_reply;

const EVENT_OUTPUT: &str = "o";
const EVENT_INPUT_MARKER: &str = "m";
const RECORD_DIR: &str = "recordings";

struct ActiveRecording {
    started: Instant,
    record_id: String,
    file: SyncMutex<File>,
    had_input: AtomicBool,
}

fn create_active_map() -> Arc<DashMap<String, Arc<ActiveRecording>>> {
    Arc::new(DashMap::new())
}

#[derive(Clone, Service)]
pub struct SessionRecordService {
    #[inject(component)]
    app_config: AppConfig,

    #[inject(component)]
    session_record_dao: SessionRecordDao,

    #[inject(func = create_active_map())]
    active: Arc<DashMap<String, Arc<ActiveRecording>>>,
}

impl SessionRecordService {
    pub async fn start(&self, session_id: &str, protocol: &str, device_name: &str, width: u16, height: u16) {
        if let Err(error) = self.do_start(session_id, protocol, device_name, width, height).await {
            warn!("[record] 会话录像启动失败 session_id={} error={}", session_id, error);
        }
    }

    pub fn append(&self, session_id: &str, data: &[u8]) {
        let Some(recording) = self.active.get(session_id).map(|entry| Arc::clone(entry.value())) else {
            return;
        };
        Self::write_event(session_id, &recording, EVENT_OUTPUT, String::from_utf8_lossy(data).as_ref());
    }

    pub fn mark_input(&self, session_id: &str, data: &str) {
        if is_terminal_reply(data) {
            return;
        }
        let Some(recording) = self.active.get(session_id).map(|entry| Arc::clone(entry.value())) else {
            return;
        };
        recording.had_input.store(true, Ordering::Relaxed);
        Self::write_event(session_id, &recording, EVENT_INPUT_MARKER, "");
    }

    fn write_event(session_id: &str, recording: &ActiveRecording, kind: &str, payload: &str) {
        let elapsed = recording.started.elapsed().as_secs_f64();
        let event = json!([elapsed, kind, payload]);
        let mut line = event.to_string();
        line.push('\n');
        let mut file = recording.file.lock();
        if let Err(error) = file.write_all(line.as_bytes()) {
            warn!("[record] 会话录像写入失败 session_id={} error={}", session_id, error);
        }
    }

    pub async fn stop(&self, session_id: &str) {
        let Some((_, recording)) = self.active.remove(session_id) else {
            return;
        };
        if let Err(error) = self.finish_record(&recording).await {
            warn!("[record] 会话录像收尾失败 session_id={} error={}", session_id, error);
        }
    }

    pub async fn list(&self) -> anyhow::Result<Vec<SessionRecord>> {
        let rows = self.session_record_dao.find_all().await?;
        let mut records: Vec<SessionRecord> = rows.into_iter().map(|ptr| ptr.read().clone()).filter(|record| record.ended_at > 0 && record.size_bytes > 0).collect();
        records.sort_by(|left, right| right.started_at.cmp(&left.started_at));
        Ok(records)
    }

    pub async fn delete_batch(&self, ids: Vec<String>) -> anyhow::Result<usize> {
        let mut removed = 0usize;
        for id in ids {
            if id.trim().is_empty() {
                continue;
            }
            self.delete(id).await?;
            removed += 1;
        }
        Ok(removed)
    }

    pub async fn delete_before(&self, started_before_millis: i64) -> anyhow::Result<usize> {
        let rows = self.session_record_dao.find_all().await?;
        let targets: Vec<String> = rows
            .into_iter()
            .filter_map(|ptr| {
                let record = ptr.read();
                if record.started_at < started_before_millis {
                    Some(record.id.clone())
                } else {
                    None
                }
            })
            .collect();
        self.delete_batch(targets).await
    }

    pub async fn content(&self, id: String) -> anyhow::Result<String> {
        let record = self.find_record(id).await?;
        let path = self.record_dir()?.join(record.file_name.as_str());
        std::fs::read_to_string(&path).with_context(|| format!("read record file {:?}", path))
    }

    pub async fn delete(&self, id: String) -> anyhow::Result<()> {
        let record = self.find_record(id.clone()).await?;
        let path = self.record_dir()?.join(record.file_name.as_str());
        if path.exists() {
            std::fs::remove_file(&path).with_context(|| format!("remove record file {:?}", path))?;
        }
        self.session_record_dao.delete_by_id(id).await?;
        Ok(())
    }

    async fn find_record(&self, id: String) -> anyhow::Result<SessionRecord> {
        let ptr: Shared<SessionRecord> = self.session_record_dao.find_by_id(id.clone()).await?.ok_or_else(|| anyhow!("session record not found: {}", id))?;
        let record = ptr.read().clone();
        Ok(record)
    }

    fn record_dir(&self) -> anyhow::Result<PathBuf> {
        let dir = self.app_config.data_dir().join(RECORD_DIR);
        std::fs::create_dir_all(&dir).with_context(|| format!("create record dir {:?}", dir))?;
        Ok(dir)
    }

    async fn do_start(&self, session_id: &str, protocol: &str, device_name: &str, width: u16, height: u16) -> anyhow::Result<()> {
        self.stop(session_id).await;

        let record_id = uuid();
        let file_name = format!("{}.cast", record_id);
        let mut record = SessionRecord::default();
        record.id = record_id.clone();
        record.protocol = protocol.to_string();
        record.device_name = device_name.to_string();
        record.file_name = file_name.clone();
        record.width = i32::from(width);
        record.height = i32::from(height);
        record.started_at = now_millis();
        self.session_record_dao.insert(record).await?;

        let path = self.record_dir()?.join(file_name.as_str());
        let mut file = File::create(&path).with_context(|| format!("create record file {:?}", path))?;
        let header = json!({
            "version": 2,
            "width": width,
            "height": height,
            "timestamp": now_millis() / 1000,
            "title": format!("{} · {}", device_name, protocol),
        });
        let mut line = header.to_string();
        line.push('\n');
        file.write_all(line.as_bytes())?;

        let recording = Arc::new(ActiveRecording { record_id, started: Instant::now(), file: SyncMutex::new(file), had_input: AtomicBool::new(false) });
        self.active.insert(session_id.to_string(), recording);
        Ok(())
    }

    async fn finish_record(&self, recording: &ActiveRecording) -> anyhow::Result<()> {
        {
            let mut file = recording.file.lock();
            file.flush()?;
        }
        let Some(ptr) = self.session_record_dao.find_by_id(recording.record_id.clone()).await? else {
            return Ok(());
        };
        let mut record = ptr.read().clone();
        record.ended_at = now_millis();
        let path = self.record_dir()?.join(record.file_name.as_str());
        let file_len = std::fs::metadata(&path)?.len();
        record.size_bytes = i64::try_from(file_len)?;
        if !recording.had_input.load(Ordering::Relaxed) {
            if path.exists() {
                std::fs::remove_file(&path).with_context(|| format!("remove idle record file {:?}", path))?;
            }
            self.session_record_dao.delete_by_id(record.id.clone()).await?;
            return Ok(());
        }
        self.session_record_dao.update(record).await?;
        Ok(())
    }
}
