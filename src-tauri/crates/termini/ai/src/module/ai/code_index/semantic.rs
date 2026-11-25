use std::fs;
use sha2::Digest;
use sha2::Sha256;
use std::io::Write;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use anyhow::{anyhow, Context, Result};
use std::thread::available_parallelism;
use crate::module::ai::code_index::extract::{extract_file, Symbol};
use crate::module::ai::code_index::walker::{is_data_language, language_for_path};

pub const SUMMARY_LINES: usize = 10;
pub const INDEX_DIR: &str = "code_index";

const INDEX_VERSION: u32 = 1;
const DIR_ID_CHARS: usize = 16;
const META_FILE: &str = "meta.json";
const CHUNK_MAX_CHARS: usize = 1200;
const TEXT_WINDOW_LINES: usize = 40;
const DATA_SUMMARY_LINES: usize = 30;
const MAX_CHUNK_THREADS: usize = 8;
const CHUNK_FILE: &str = "chunks.jsonl";
const VECTOR_FILE: &str = "vectors.bin";
const MANIFEST_FILE: &str = "manifest.json";

const HASH_CHARS: usize = 16;
const CACHE_META_FILE: &str = "cache_meta.json";
const CACHE_HASH_FILE: &str = "cache_hash.bin";
const CACHE_VECTOR_FILE: &str = "cache_vec.bin";

#[derive(Clone, Serialize, Deserialize)]
struct CacheMeta {
    version: u32,
    provide_id: String,
    dim: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct IndexMeta {
    pub version: u32,
    pub root: String,
    pub provide_id: String,
    pub dim: usize,
    pub summary_lines: usize,
    pub built_at: i64,
    pub chunk_count: usize,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileStamp {
    pub mtime_ms: i64,
    pub size: u64,
}

#[derive(Default, Serialize, Deserialize)]
pub struct IndexManifest {
    pub files: BTreeMap<String, FileStamp>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChunkRow {
    pub hash: String,
    pub path: String,
    pub symbol: String,
    pub start_line: usize,
    pub end_line: usize,
    pub preview: String,
}

pub struct ChunkInput {
    pub row: ChunkRow,
    pub text: String,
}

pub fn index_dir(data_dir: &Path, root: &Path) -> PathBuf {
    let key = root.to_string_lossy().replace('\\', "/").to_lowercase();
    let digest = Sha256::digest(key.as_bytes());
    let id: String = digest.iter().map(|byte| format!("{:02x}", byte)).collect();
    data_dir.join(INDEX_DIR).join(&id[..DIR_ID_CHARS])
}

pub fn load_meta(dir: &Path) -> Option<IndexMeta> {
    let text = fs::read_to_string(dir.join(META_FILE)).ok()?;
    serde_json::from_str::<IndexMeta>(&text).ok()
}

pub fn load_manifest(dir: &Path) -> Option<IndexManifest> {
    let text = fs::read_to_string(dir.join(MANIFEST_FILE)).ok()?;
    serde_json::from_str::<IndexManifest>(&text).ok()
}

pub fn load_rows(dir: &Path) -> Result<Vec<ChunkRow>> {
    let path = dir.join(CHUNK_FILE);
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        rows.push(serde_json::from_str::<ChunkRow>(line).with_context(|| format!("parse chunk row in {}", path.display()))?);
    }
    Ok(rows)
}

pub fn load_vectors(dir: &Path, dim: usize, count: usize) -> Result<Vec<i8>> {
    let path = dir.join(VECTOR_FILE);
    let bytes = fs::read(&path).with_context(|| format!("read {}", path.display()))?;
    if bytes.len() != dim * count {
        return Err(anyhow!("vector size mismatch: expect {} got {}", dim * count, bytes.len()));
    }
    Ok(bytes.into_iter().map(|byte| byte as i8).collect())
}

pub fn save_index(dir: &Path, meta: &IndexMeta, manifest: &IndexManifest, rows: &[ChunkRow], vectors: &[i8]) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let mut chunk_text = String::new();
    for row in rows {
        chunk_text.push_str(&serde_json::to_string(row).context("encode chunk row")?);
        chunk_text.push('\n');
    }
    fs::write(dir.join(CHUNK_FILE), chunk_text).with_context(|| format!("write {}", CHUNK_FILE))?;
    let bytes: Vec<u8> = vectors.iter().map(|value| *value as u8).collect();
    fs::write(dir.join(VECTOR_FILE), bytes).with_context(|| format!("write {}", VECTOR_FILE))?;
    fs::write(dir.join(MANIFEST_FILE), serde_json::to_string(manifest).context("encode manifest")?).with_context(|| format!("write {}", MANIFEST_FILE))?;
    fs::write(dir.join(META_FILE), serde_json::to_string(meta).context("encode meta")?).with_context(|| format!("write {}", META_FILE))?;
    Ok(())
}

pub fn load_vector_cache(dir: &Path, provide_id: &str, dim: usize) -> BTreeMap<String, Vec<i8>> {
    let mut cache = BTreeMap::new();
    if dim == 0 || !cache_meta_matches(dir, provide_id, dim) {
        return cache;
    }
    let Ok(hash_bytes) = fs::read(dir.join(CACHE_HASH_FILE)) else {
        return cache;
    };
    let Ok(vector_bytes) = fs::read(dir.join(CACHE_VECTOR_FILE)) else {
        return cache;
    };
    let count = (hash_bytes.len() / HASH_CHARS).min(vector_bytes.len() / dim);
    for index in 0..count {
        let hash_slice = &hash_bytes[index * HASH_CHARS..(index + 1) * HASH_CHARS];
        let Ok(hash) = std::str::from_utf8(hash_slice) else {
            continue;
        };
        let offset = index * dim;
        cache.insert(hash.to_string(), vector_bytes[offset..offset + dim].iter().map(|byte| *byte as i8).collect());
    }
    cache
}

pub fn append_vector_cache(dir: &Path, entries: &[(String, Vec<i8>)]) -> Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut hash_buf = Vec::with_capacity(entries.len() * HASH_CHARS);
    let mut vector_buf = Vec::new();
    for (hash, vector) in entries {
        if hash.len() != HASH_CHARS {
            return Err(anyhow!("chunk hash length is not {}: {}", HASH_CHARS, hash));
        }
        hash_buf.extend_from_slice(hash.as_bytes());
        vector_buf.extend(vector.iter().map(|value| *value as u8));
    }
    append_file(&dir.join(CACHE_VECTOR_FILE), &vector_buf)?;
    append_file(&dir.join(CACHE_HASH_FILE), &hash_buf)?;
    Ok(())
}

pub fn reset_vector_cache(dir: &Path, provide_id: &str, dim: usize) -> Result<()> {
    fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let _ = fs::remove_file(dir.join(CACHE_HASH_FILE));
    let _ = fs::remove_file(dir.join(CACHE_VECTOR_FILE));
    let meta = CacheMeta { version: INDEX_VERSION, provide_id: provide_id.to_string(), dim };
    fs::write(dir.join(CACHE_META_FILE), serde_json::to_string(&meta).context("encode cache meta")?).with_context(|| format!("write {}", CACHE_META_FILE))?;
    Ok(())
}

pub fn cache_meta_matches(dir: &Path, provide_id: &str, dim: usize) -> bool {
    let Ok(text) = fs::read_to_string(dir.join(CACHE_META_FILE)) else {
        return false;
    };
    let Ok(meta) = serde_json::from_str::<CacheMeta>(&text) else {
        return false;
    };
    meta.version == INDEX_VERSION && meta.provide_id == provide_id && meta.dim == dim
}

fn append_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = fs::OpenOptions::new().create(true).append(true).open(path).with_context(|| format!("open {}", path.display()))?;
    file.write_all(bytes).with_context(|| format!("append {}", path.display()))?;
    Ok(())
}

pub fn current_manifest(root: &Path, files: &[PathBuf]) -> IndexManifest {
    let mut manifest = IndexManifest::default();
    for file in files {
        let Ok(meta) = fs::metadata(file) else {
            continue;
        };
        let Ok(modified) = meta.modified() else {
            continue;
        };
        let Ok(elapsed) = modified.duration_since(std::time::UNIX_EPOCH) else {
            continue;
        };
        manifest.files.insert(relative_of(root, file), FileStamp { mtime_ms: elapsed.as_millis() as i64, size: meta.len() });
    }
    manifest
}

pub fn meta_matches(meta: &IndexMeta, provide_id: &str, dim: usize) -> bool {
    meta.version == INDEX_VERSION && meta.provide_id == provide_id && meta.dim == dim && meta.summary_lines == SUMMARY_LINES
}

pub fn build_meta(root: &Path, provide_id: &str, dim: usize, chunk_count: usize, built_at: i64) -> IndexMeta {
    IndexMeta { version: INDEX_VERSION, root: root.to_string_lossy().to_string(), provide_id: provide_id.to_string(), dim, summary_lines: SUMMARY_LINES, built_at, chunk_count }
}

pub fn build_chunks(root: &Path, files: &[PathBuf]) -> Vec<ChunkInput> {
    let threads = available_parallelism().map(|value| value.get()).unwrap_or(4).clamp(1, MAX_CHUNK_THREADS);
    let shard = files.len().div_ceil(threads).max(1);
    let mut chunks = Vec::new();
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for part in files.chunks(shard) {
            handles.push(scope.spawn(move || {
                let mut local = Vec::new();
                for file in part {
                    build_file_chunks(root, file, &mut local);
                }
                local
            }));
        }
        for handle in handles {
            match handle.join() {
                Ok(local) => chunks.extend(local),
                Err(_) => continue,
            }
        }
    });
    chunks
}

fn build_file_chunks(root: &Path, file: &Path, out: &mut Vec<ChunkInput>) {
    let Ok(content) = fs::read_to_string(file) else {
        return;
    };
    let relative = relative_of(root, file);
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return;
    }
    if language_for_path(file).map(is_data_language).unwrap_or(false) {
        out.push(summary_chunk(&relative, &lines, DATA_SUMMARY_LINES));
        return;
    }
    out.push(summary_chunk(&relative, &lines, SUMMARY_LINES));
    if language_for_path(file).is_some() {
        out.extend(symbol_chunks(file, &relative, &lines));
        return;
    }
    out.extend(window_chunks(&relative, &lines));
}

pub fn quantize(vector: &[f32]) -> Vec<i8> {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm <= f32::EPSILON {
        return vec![0i8; vector.len()];
    }
    vector.iter().map(|value| ((value / norm) * 127.0).round().clamp(-127.0, 127.0) as i8).collect()
}

pub fn search(query: &[f32], vectors: &[i8], dim: usize, top_k: usize) -> Vec<(usize, f32)> {
    search_filtered(query, vectors, dim, top_k, None)
}

pub fn search_filtered(query: &[f32], vectors: &[i8], dim: usize, top_k: usize, allow: Option<&[bool]>) -> Vec<(usize, f32)> {
    let norm = query.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm <= f32::EPSILON || dim == 0 {
        return Vec::new();
    }
    let unit: Vec<f32> = query.iter().map(|value| value / norm).collect();
    let count = vectors.len() / dim;
    let mut scored: Vec<(usize, f32)> = Vec::with_capacity(count);
    for index in 0..count {
        if allow.is_some_and(|mask| !mask.get(index).copied().unwrap_or(false)) {
            continue;
        }
        let offset = index * dim;
        let mut score = 0f32;
        for position in 0..dim {
            score += unit[position] * (vectors[offset + position] as f32 / 127.0);
        }
        scored.push((index, score));
    }
    scored.sort_by(|left, right| right.1.total_cmp(&left.1));
    scored.truncate(top_k);
    scored
}

fn relative_of(root: &Path, file: &Path) -> String {
    let stripped = file.strip_prefix(root).unwrap_or(file);
    let value = if stripped.as_os_str().is_empty() { file } else { stripped };
    value.to_string_lossy().replace('\\', "/")
}

fn summary_chunk(relative: &str, lines: &[&str], head_lines: usize) -> ChunkInput {
    let end = head_lines.min(lines.len());
    let body = lines[..end].join("\n");
    let text = clamp_chars(&format!("{}\n{}", relative, body));
    ChunkInput { row: row_of(relative, "file", 1, end, &text), text }
}

fn symbol_chunks(file: &Path, relative: &str, lines: &[&str]) -> Vec<ChunkInput> {
    let Ok(symbols) = extract_file(file, relative) else {
        return Vec::new();
    };
    let mut defs: Vec<&Symbol> = symbols.iter().filter(|item| item.is_def).collect();
    defs.sort_by_key(|item| item.start_line);
    let mut chunks = Vec::new();
    let mut covered_until = 0usize;
    for symbol in defs {
        if symbol.start_line == 0 || symbol.start_line > lines.len() {
            continue;
        }
        if symbol.start_line < covered_until {
            continue;
        }
        let start = symbol.start_line - 1;
        let end = (start + SUMMARY_LINES).min(lines.len());
        covered_until = symbol.start_line + SUMMARY_LINES;
        let body = lines[start..end].join("\n");
        let text = clamp_chars(&format!("{}\n{} {}\n{}", relative, symbol.kind, symbol.name, body));
        chunks.push(ChunkInput { row: row_of(relative, &symbol.name, symbol.start_line, end, &text), text });
    }
    chunks
}

fn window_chunks(relative: &str, lines: &[&str]) -> Vec<ChunkInput> {
    let mut chunks = Vec::new();
    let mut start = 0usize;
    while start < lines.len() {
        let end = (start + TEXT_WINDOW_LINES).min(lines.len());
        let body = lines[start..end].join("\n");
        if !body.trim().is_empty() {
            let text = clamp_chars(&format!("{}\n{}", relative, body));
            chunks.push(ChunkInput { row: row_of(relative, "text", start + 1, end, &text), text });
        }
        start = end;
    }
    chunks
}

fn row_of(relative: &str, symbol: &str, start_line: usize, end_line: usize, text: &str) -> ChunkRow {
    let digest = Sha256::digest(text.as_bytes());
    let hash: String = digest.iter().take(8).map(|byte| format!("{:02x}", byte)).collect();
    ChunkRow { hash, path: relative.to_string(), symbol: symbol.to_string(), start_line, end_line, preview: clamp_preview(text) }
}

fn clamp_chars(text: &str) -> String {
    if text.chars().count() <= CHUNK_MAX_CHARS {
        return text.to_string();
    }
    text.chars().take(CHUNK_MAX_CHARS).collect()
}

fn clamp_preview(text: &str) -> String {
    let single = text.replace('\n', " ⏎ ");
    if single.chars().count() <= 160 {
        return single;
    }
    single.chars().take(160).collect()
}
