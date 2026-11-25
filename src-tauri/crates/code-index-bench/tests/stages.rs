use std::fs;
use std::time::Instant;
use std::path::{Path, PathBuf};
use std::collections::{BTreeMap, HashMap, HashSet};
use code_index_bench::module::ai::code_index::semantic;
use code_index_bench::module::ai::code_index::extract::extract_file;
use code_index_bench::module::ai::code_index::walker::{language_for_path, walk_index_files};

const DIM: usize = 1024;
const BATCH: usize = 256;
const CONCURRENCY: usize = 12;

/// 实测端到端吞吐(块/秒)。经服务端代理打 Qwen3-Embedding-0.6B, 每块约 500 字符。
/// 固定 12288 块跑完的公平对比:
///   并发 12 x4轮  35.6s  345 块/秒
///   并发 24 x2轮  37.1s  332 块/秒
///   并发 48 x1轮  34.3s  358 块/秒
/// 三者在噪声范围内 —— 上游已饱和, 加并发不再有收益, 客户端 12 并发就够。
/// 代理层不是瓶颈: 单请求过代理只比直连多约 10%, 并发下因服务端出网更好反而略快。
const MEASURED_THROUGHPUT: f64 = 345.0;

fn default_roots() -> Vec<PathBuf> {
    // crates/code-index-bench -> 上溯四级到仓库根
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..").join("archive");
    vec![base.join("codex-main"), base.join("grok-build-main")]
}

fn resolve_roots() -> Vec<PathBuf> {
    let Ok(raw) = std::env::var("BENCH_ROOTS") else {
        return default_roots();
    };
    raw.split(';').map(|item| item.trim()).filter(|item| !item.is_empty()).map(PathBuf::from).collect()
}

fn ms(start: Instant) -> u128 {
    start.elapsed().as_millis()
}

fn span_of(row: &semantic::ChunkRow) -> usize {
    row.end_line.saturating_sub(row.start_line) + 1
}

#[test]
#[ignore]
fn bench_index_stages() {
    for root in resolve_roots() {
        if !root.is_dir() {
            println!("[skip] 目录不存在: {}", root.display());
            continue;
        }
        println!("\n================ {} ================", root.display());

        let started = Instant::now();
        let files = walk_index_files(&root, &[], false).expect("walk");
        println!("[1] 遍历      {:>8} ms   文件 {}", ms(started), files.len());

        // 语法解析单独计时: 它是切块里最重的一环, 要先知道占比
        let started = Instant::now();
        let mut parsed = 0usize;
        let mut defs = 0usize;
        for file in &files {
            if language_for_path(file).is_none() {
                continue;
            }
            let relative = file.strip_prefix(&root).unwrap_or(file).to_string_lossy().replace('\\', "/");
            if let Ok(symbols) = extract_file(file, &relative) {
                parsed += 1;
                defs += symbols.iter().filter(|item| item.is_def).count();
            }
        }
        println!("[2] 语法解析  {:>8} ms   文件 {}  定义 {}", ms(started), parsed, defs);

        let started = Instant::now();
        let manifest = semantic::current_manifest(&root, &files);
        println!("[3] 清单      {:>8} ms   条目 {}", ms(started), manifest.files.len());

        let started = Instant::now();
        let chunks = semantic::build_chunks(&root, &files);
        println!("[4] 切块      {:>8} ms   块 {}", ms(started), chunks.len());

        report_symbol_kinds(&root, &files);
        report_hotspots(&chunks);
        report_chunk_shape(&chunks);
        report_merge_potential(&chunks);
        report_cache_io(&chunks);
        report_search(chunks.len());
        report_embed_estimate(&chunks);
    }
}

/// 定义按种类分布。块数 = 定义数, 所以哪类定义最多, 哪类就最该被审视。
fn report_symbol_kinds(root: &Path, files: &[PathBuf]) {
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for file in files {
        if language_for_path(file).is_none() {
            continue;
        }
        let relative = file.strip_prefix(root).unwrap_or(file).to_string_lossy().replace('\\', "/");
        let Ok(symbols) = extract_file(file, &relative) else {
            continue;
        };
        for symbol in symbols.iter().filter(|item| item.is_def) {
            *kinds.entry(symbol.kind.clone()).or_insert(0) += 1;
        }
    }
    let total: usize = kinds.values().sum();
    let mut rows: Vec<(&String, &usize)> = kinds.iter().collect();
    rows.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    let text: Vec<String> = rows.iter().map(|(kind, count)| format!("{}={} ({:.0}%)", kind, count, **count as f64 * 100.0 / total.max(1) as f64)).collect();
    println!("    定义种类  {}", text.join("  "));
}

fn report_hotspots(chunks: &[semantic::ChunkInput]) {
    let mut by_dir: HashMap<&str, (usize, usize)> = HashMap::new();
    let mut by_ext: HashMap<&str, (usize, usize)> = HashMap::new();
    for chunk in chunks {
        let path = chunk.row.path.as_str();
        let chars = chunk.text.chars().count();
        let dir = path.split('/').next().unwrap_or(".");
        let entry = by_dir.entry(dir).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += chars;
        let ext = path.rsplit_once('.').map(|(_, ext)| ext).unwrap_or("(none)");
        let entry = by_ext.entry(ext).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += chars;
    }
    let total_chars: usize = chunks.iter().map(|chunk| chunk.text.chars().count()).sum();

    let mut dirs: Vec<(&&str, &(usize, usize))> = by_dir.iter().collect();
    dirs.sort_by_key(|(_, value)| std::cmp::Reverse(value.1));
    println!("    顶层目录  (块数 / 字符占比)");
    for (dir, (count, chars)) in dirs.iter().take(8) {
        println!("        {:<28} {:>7} 块  {:>5.1}%", dir, count, *chars as f64 * 100.0 / total_chars.max(1) as f64);
    }

    let mut exts: Vec<(&&str, &(usize, usize))> = by_ext.iter().collect();
    exts.sort_by_key(|(_, value)| std::cmp::Reverse(value.1));
    let text: Vec<String> = exts.iter().take(10).map(|(ext, (count, chars))| format!("{}={}块/{:.0}%", ext, count, *chars as f64 * 100.0 / total_chars.max(1) as f64)).collect();
    println!("    扩展名    {}", text.join("  "));
}

fn report_merge_potential(chunks: &[semantic::ChunkInput]) {
    let mut by_file: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    let mut non_symbol = 0usize;
    for chunk in chunks {
        if chunk.row.symbol == "file" || chunk.row.symbol == "text" {
            non_symbol += 1;
            continue;
        }
        by_file.entry(chunk.row.path.as_str()).or_default().push(chunk.row.start_line);
    }
    for window in [10usize, 20, 40] {
        let mut merged = 0usize;
        for starts in by_file.values() {
            let mut sorted = starts.clone();
            sorted.sort_unstable();
            let mut cursor: Option<usize> = None;
            for start in sorted {
                match cursor {
                    Some(last) if start < last + window => {}
                    _ => {
                        merged += 1;
                        cursor = Some(start);
                    }
                }
            }
        }
        let total = merged + non_symbol;
        let pct = if chunks.is_empty() { 0.0 } else { total as f64 * 100.0 / chunks.len() as f64 };
        println!("    合并{:>3}行窗口 符号块 {} -> {}, 总块 {} ({:.0}% of 原始)", window, chunks.len() - non_symbol, merged, total, pct);
    }
}

fn report_chunk_shape(chunks: &[semantic::ChunkInput]) {
    let mut unique: HashSet<&str> = HashSet::new();
    let mut chars_total = 0usize;
    let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    let mut span_hist: BTreeMap<&str, usize> = BTreeMap::new();

    for chunk in chunks {
        chars_total += chunk.text.chars().count();
        unique.insert(chunk.row.hash.as_str());
        let kind = match chunk.row.symbol.as_str() {
            "file" => "file",
            "text" => "text",
            _ => "symbol",
        };
        *by_kind.entry(kind).or_insert(0) += 1;
        let span = span_of(&chunk.row);
        let bucket = if span <= 2 {
            "<=2行"
        } else if span <= 5 {
            "3-5行"
        } else if span <= 10 {
            "6-10行"
        } else {
            ">10行"
        };
        *span_hist.entry(bucket).or_insert(0) += 1;
    }

    let dup = chunks.len() - unique.len();
    let dup_pct = if chunks.is_empty() { 0.0 } else { dup as f64 * 100.0 / chunks.len() as f64 };
    println!("    去重      唯一 {}  重复 {} ({:.1}%)", unique.len(), dup, dup_pct);
    println!("    文本量    共 {} 万字符  平均 {} 字符/块", chars_total / 10000, chars_total.checked_div(chunks.len()).unwrap_or(0));
    println!("    构成      {:?}", by_kind);
    println!("    符号跨度  {:?}", span_hist);

    for floor in [3usize, 5, 8] {
        let mut kept: HashSet<&str> = HashSet::new();
        for chunk in chunks {
            let is_symbol = chunk.row.symbol != "file" && chunk.row.symbol != "text";
            if is_symbol && span_of(&chunk.row) < floor {
                continue;
            }
            kept.insert(chunk.row.hash.as_str());
        }
        let pct = if chunks.is_empty() { 0.0 } else { kept.len() as f64 * 100.0 / chunks.len() as f64 };
        println!("    过滤<{}行 去重后剩 {} 块 ({:.1}% of 原始 {})", floor, kept.len(), pct, chunks.len());
    }
}

fn report_cache_io(chunks: &[semantic::ChunkInput]) {
    let sample = chunks.len().min(20000);
    if sample == 0 {
        return;
    }
    let fake: Vec<f32> = (0..DIM).map(|index| (index as f32 % 7.0) - 3.0).collect();

    let started = Instant::now();
    let mut entries = Vec::with_capacity(sample);
    for chunk in chunks.iter().take(sample) {
        entries.push((chunk.row.hash.clone(), semantic::quantize(&fake)));
    }
    let quantize_ms = ms(started);

    let dir = std::env::temp_dir().join("termini-bench-cache");
    let _ = fs::remove_dir_all(&dir);
    semantic::reset_vector_cache(&dir, "bench", DIM).expect("reset cache");

    let started = Instant::now();
    for batch in entries.chunks(BATCH) {
        semantic::append_vector_cache(&dir, batch).expect("append cache");
    }
    let append_ms = ms(started);

    let started = Instant::now();
    let loaded = semantic::load_vector_cache(&dir, "bench", DIM);
    let load_ms = ms(started);

    println!("[5] 缓存IO    量化 {} 块 {} ms | 分批落盘 {} ms | 回读 {} ms (命中 {})", sample, quantize_ms, append_ms, load_ms, loaded.len());
    let _ = fs::remove_dir_all(&dir);
}

fn report_search(count: usize) {
    if count == 0 {
        return;
    }
    let vectors: Vec<i8> = vec![1i8; count * DIM];
    let query: Vec<f32> = (0..DIM).map(|index| (index as f32 % 5.0) - 2.0).collect();
    let started = Instant::now();
    let hits = semantic::search(&query, &vectors, DIM, 20);
    println!("[6] 检索      {} 块暴力点积 {} ms (命中 {})", count, ms(started), hits.len());
}

fn report_embed_estimate(chunks: &[semantic::ChunkInput]) {
    let mut unique: HashSet<&str> = HashSet::new();
    for chunk in chunks {
        unique.insert(chunk.row.hash.as_str());
    }
    let batches = unique.len().div_ceil(BATCH);
    let seconds = unique.len() as f64 / MEASURED_THROUGHPUT;
    println!("[7] 编码估算  去重后 {} 块 => {} 批 (batch={}, 并发{}) => 约 {:.0} 秒 / {:.1} 分钟", unique.len(), batches, BATCH, CONCURRENCY, seconds, seconds / 60.0);
    println!("               基于实测上游吞吐 {} 块/秒 (Qwen3-Embedding-0.6B, 并发12 x batch256)", MEASURED_THROUGHPUT);
}

#[test]
#[ignore]
fn bench_duplicate_sources() {
    for root in resolve_roots() {
        if !root.is_dir() {
            continue;
        }
        let files = walk_index_files(&root, &[], false).expect("walk");
        let chunks = semantic::build_chunks(&root, &files);
        let mut groups: HashMap<&str, Vec<&semantic::ChunkRow>> = HashMap::new();
        for chunk in &chunks {
            groups.entry(chunk.row.hash.as_str()).or_default().push(&chunk.row);
        }
        let mut hot: Vec<(&str, usize, &semantic::ChunkRow)> = groups.iter().filter(|(_, rows)| rows.len() > 1).map(|(hash, rows)| (*hash, rows.len(), rows[0])).collect();
        hot.sort_by_key(|(_, count, _)| std::cmp::Reverse(*count));
        println!("\n---- {} 重复块 Top15 ----", root.display());
        for (hash, count, row) in hot.iter().take(15) {
            println!("  x{:<4} {} [{}] {}", count, hash, row.symbol, row.preview.chars().take(80).collect::<String>());
        }
    }
}
