use std::fs;
use anyhow::Result;
use std::sync::Arc;
use dashmap::DashMap;
use summer::plugin::Service;
use termini_macros::base_app;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use crate::base::base_app::BaseApp;
use std::thread::available_parallelism;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime};
use crate::module::ai::code_index::index_guard::normalize;
use crate::module::ai::code_index::walker::walk_source_files;
use crate::module::ai::code_index::extract::{extract_file, Symbol};

const WALK_TTL: Duration = Duration::from_secs(60);
const MAX_PARSE_WORKERS: usize = 8;

#[derive(Debug, Clone)]
struct FileEntry {
    mtime: SystemTime,
    symbols: Vec<Symbol>,
}

struct WalkCache {
    files: Arc<Vec<PathBuf>>,
    walked_at: Instant,
}

struct ViewCache {
    snapshot: Arc<Vec<Symbol>>,
    version: u64,
}

#[base_app(sort = 762)]
#[derive(Clone, Service)]
pub struct CodeIndex {
    #[inject(func = create_files())]
    files: Arc<DashMap<PathBuf, FileEntry>>,

    #[inject(func = create_walks())]
    walks: Arc<DashMap<String, WalkCache>>,

    #[inject(func = create_views())]
    views: Arc<DashMap<String, ViewCache>>,

    #[inject(func = create_version())]
    version: Arc<AtomicU64>,
}

fn create_files() -> Arc<DashMap<PathBuf, FileEntry>> {
    Arc::new(DashMap::new())
}

fn create_walks() -> Arc<DashMap<String, WalkCache>> {
    Arc::new(DashMap::new())
}

fn create_views() -> Arc<DashMap<String, ViewCache>> {
    Arc::new(DashMap::new())
}

fn create_version() -> Arc<AtomicU64> {
    Arc::new(AtomicU64::new(0))
}

impl BaseApp for CodeIndex {
    fn name(&self) -> &'static str {
        "CodeIndex"
    }
}

impl CodeIndex {
    pub fn symbols_for_view(&self, base: &Path, view_root: &Path) -> Result<Arc<Vec<Symbol>>> {
        let files = self.sync_base(base)?;
        let key = format!("{}\u{1f}{}", normalize(base), normalize(view_root));
        let version = self.version.load(Ordering::Acquire);
        if let Some(view) = self.views.get(&key) {
            if view.version == version {
                return Ok(view.snapshot.clone());
            }
        }
        let snapshot = Arc::new(self.build_view(&files, view_root));
        self.views.insert(key, ViewCache { snapshot: snapshot.clone(), version });
        Ok(snapshot)
    }

    pub fn symbols_for_root(&self, root: &Path) -> Result<Arc<Vec<Symbol>>> {
        self.symbols_for_view(root, root)
    }

    pub fn symbols_for_bases(&self, request: &Path, bases: &[PathBuf]) -> Result<Arc<Vec<Symbol>>> {
        if let [single] = bases {
            return self.symbols_for_view(single, request);
        }
        let mut merged = Vec::new();
        for base in bases {
            merged.extend(self.symbols_for_view(base, request)?.iter().cloned());
        }
        Ok(Arc::new(merged))
    }

    fn sync_base(&self, base: &Path) -> Result<Arc<Vec<PathBuf>>> {
        let key = normalize(base);
        let cached = self.walks.get(&key).filter(|entry| entry.walked_at.elapsed() < WALK_TTL).map(|entry| entry.files.clone());
        let files = match cached {
            Some(files) => files,
            None => {
                let files = Arc::new(walk_source_files(base)?);
                self.walks.insert(key, WalkCache { files: files.clone(), walked_at: Instant::now() });
                files
            }
        };

        let mut stale = Vec::new();
        let mut seen = HashSet::with_capacity(files.len());
        for path in files.iter() {
            seen.insert(path.clone());
            let Ok(mtime) = fs::metadata(path).and_then(|meta| meta.modified()) else {
                continue;
            };
            match self.files.get(path) {
                Some(entry) if entry.mtime == mtime => {}
                _ => stale.push(path.clone()),
            }
        }
        let mut changed = !stale.is_empty();
        if changed {
            for (path, entry) in parse_files(&stale) {
                self.files.insert(path, entry);
            }
        }
        let dropped: Vec<PathBuf> = self.files.iter().filter(|entry| !seen.contains(entry.key()) && is_within(base, entry.key())).map(|entry| entry.key().clone()).collect();
        if !dropped.is_empty() {
            changed = true;
            for path in dropped {
                self.files.remove(&path);
            }
        }
        if changed {
            self.version.fetch_add(1, Ordering::Release);
        }
        Ok(files)
    }

    fn build_view(&self, files: &[PathBuf], view_root: &Path) -> Vec<Symbol> {
        let mut out = Vec::new();
        for path in files {
            if !is_within_or_eq(view_root, path) {
                continue;
            }
            let Some(entry) = self.files.get(path) else {
                continue;
            };
            let relative = relative_of(view_root, path);
            for symbol in entry.symbols.iter() {
                out.push(Symbol { path: relative.clone(), ..symbol.clone() });
            }
        }
        out
    }
}

fn relative_of(root: &Path, file: &Path) -> String {
    let stripped = file.strip_prefix(root).unwrap_or(file);
    let value = if stripped.as_os_str().is_empty() { file } else { stripped };
    value.to_string_lossy().replace('\\', "/")
}

fn is_within(root: &Path, path: &Path) -> bool {
    let root = normalize(root);
    let path = normalize(path);
    path.starts_with(&format!("{}/", root))
}

fn is_within_or_eq(root: &Path, path: &Path) -> bool {
    let normalized_root = normalize(root);
    let normalized_path = normalize(path);
    normalized_path == normalized_root || normalized_path.starts_with(&format!("{}/", normalized_root))
}

fn parse_files(paths: &[PathBuf]) -> Vec<(PathBuf, FileEntry)> {
    let workers = available_parallelism().map(|n| n.get()).unwrap_or(4).min(MAX_PARSE_WORKERS);
    let results: Vec<(PathBuf, Option<Vec<Symbol>>)> = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        let chunk = paths.len().div_ceil(workers).max(1);
        for batch in paths.chunks(chunk) {
            let batch = batch.to_vec();
            handles.push(scope.spawn(move || {
                batch
                    .into_iter()
                    .map(|path| {
                        let absolute = path.to_string_lossy().replace('\\', "/");
                        let symbols = extract_file(&path, &absolute).ok();
                        (path, symbols)
                    })
                    .collect::<Vec<_>>()
            }));
        }
        handles.into_iter().flat_map(|handle| handle.join().unwrap_or_default()).collect()
    });
    let mut out = Vec::with_capacity(results.len());
    for (path, symbols) in results {
        let Some(symbols) = symbols else {
            continue;
        };
        let Ok(mtime) = fs::metadata(&path).and_then(|meta| meta.modified()) else {
            continue;
        };
        out.push((path, FileEntry { mtime, symbols }));
    }
    out
}

impl CodeIndex {
    pub fn invalidate(&self, root: &Path) {
        let key = normalize(root);
        self.walks.retain(|cached, _| cached != &key && !cached.starts_with(&format!("{}/", key)) && !key.starts_with(&format!("{}/", cached)));
        let dropped: Vec<PathBuf> = self.files.iter().filter(|entry| is_within_or_eq(root, entry.key())).map(|entry| entry.key().clone()).collect();
        for path in dropped {
            self.files.remove(&path);
        }
        self.version.fetch_add(1, Ordering::Release);
    }
}
