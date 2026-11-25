use std::fs;
use parking_lot::Mutex;
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};
use regex::{Regex, RegexBuilder};
use std::time::{Duration, Instant};
use ignore::{WalkBuilder, WalkState};
use std::thread::available_parallelism;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use std::sync::{Condvar, Mutex as StdMutex, OnceLock};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};

const MAX_WALK_THREADS: usize = 8;
const MAX_HITS_PER_FILE: usize = 3;
const HARD_MAX_MATCHES: usize = 200;
const DEFAULT_MAX_MATCHES: usize = 50;
pub const SEARCH_CONTEXT_LINES: usize = 3;
pub const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct SearchHit {
    pub path: PathBuf,
    pub line: usize,
    pub text: String,
    pub after: Vec<String>,
    pub before: Vec<String>,
}

pub fn resolve_root(root: &str) -> Result<PathBuf> {
    let path = resolve_search_root(root)?;
    if !path.is_dir() {
        return Err(anyhow!("root is not a directory: {}", path.display()));
    }
    Ok(path)
}

pub fn resolve_search_root(root: &str) -> Result<PathBuf> {
    let trimmed = root.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("root is empty"));
    }
    let path = PathBuf::from(trimmed);
    if !path.is_absolute() {
        return Err(anyhow!("root must be absolute: {}", trimmed));
    }
    if !path.exists() {
        return Err(anyhow!("root not found: {}", path.display()));
    }
    Ok(path)
}

struct BudgetState {
    free: usize,
    next_ticket: u64,
    now_serving: u64,
}

struct WalkBudget {
    total: usize,
    state: StdMutex<BudgetState>,
    signal: Condvar,
}

struct WalkPermit {
    budget: &'static WalkBudget,
    threads: usize,
    granted: usize,
}

impl WalkBudget {
    const WAIT_BUDGET: Duration = Duration::from_secs(3);

    fn global() -> &'static WalkBudget {
        static BUDGET: OnceLock<WalkBudget> = OnceLock::new();
        BUDGET.get_or_init(|| {
            let total = available_parallelism().map(|n| n.get()).unwrap_or(4).max(1);
            WalkBudget { total, state: StdMutex::new(BudgetState { free: total, next_ticket: 0, now_serving: 0 }), signal: Condvar::new() }
        })
    }

    fn want(&self) -> usize {
        (self.total / 4).clamp(1, MAX_WALK_THREADS)
    }

    fn acquire(&'static self) -> WalkPermit {
        let want = self.want();
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        let ticket = state.next_ticket;
        state.next_ticket += 1;
        let deadline = Instant::now() + Self::WAIT_BUDGET;
        while state.now_serving != ticket || state.free == 0 {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                if state.now_serving == ticket {
                    state.now_serving += 1;
                    self.signal.notify_all();
                }
                return WalkPermit { budget: self, threads: 1, granted: 0 };
            };
            let (next, _) = self.signal.wait_timeout(state, remaining).unwrap_or_else(|error| error.into_inner());
            state = next;
        }
        let threads = want.min(state.free);
        state.free -= threads;
        state.now_serving += 1;
        self.signal.notify_all();
        WalkPermit { budget: self, threads, granted: threads }
    }

    fn release(&self, granted: usize) {
        if granted == 0 {
            return;
        }
        let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
        state.free = (state.free + granted).min(self.total);
        self.signal.notify_all();
    }
}

impl Drop for WalkPermit {
    fn drop(&mut self) {
        self.budget.release(self.granted);
    }
}

fn walk_permit() -> WalkPermit {
    WalkBudget::global().acquire()
}

pub fn walk_source_files(root: &Path) -> Result<Vec<PathBuf>> {
    let permit = walk_permit();
    let files = Mutex::new(Vec::new());
    let failure = Mutex::new(None);
    WalkBuilder::new(root).hidden(false).git_ignore(true).git_global(true).git_exclude(true).threads(permit.threads).build_parallel().run(|| {
        Box::new(|entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    failure.lock().get_or_insert(error.to_string());
                    return WalkState::Quit;
                }
            };
            if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                return WalkState::Continue;
            }
            let path = entry.path();
            if language_for_path(path).is_none() {
                return WalkState::Continue;
            }
            let Ok(meta) = entry.metadata() else {
                return WalkState::Continue;
            };
            if meta.len() > MAX_FILE_BYTES {
                return WalkState::Continue;
            }
            files.lock().push(path.to_path_buf());
            WalkState::Continue
        })
    });
    if let Some(error) = failure.into_inner() {
        return Err(anyhow!(error));
    }
    let mut files = files.into_inner();
    files.sort();
    Ok(files)
}

pub fn search_files(root: &Path, query: &str, glob: Option<&str>, max_matches: Option<usize>) -> Result<Vec<SearchHit>> {
    let mut grouped = search_files_multi(root, std::slice::from_ref(&query.to_string()), glob, max_matches)?;
    Ok(grouped.pop().unwrap_or_default())
}

pub fn search_files_multi(root: &Path, queries: &[String], glob: Option<&str>, max_matches: Option<usize>) -> Result<Vec<Vec<SearchHit>>> {
    if queries.is_empty() {
        return Err(anyhow!("queries is empty"));
    }
    let mut regexes = Vec::with_capacity(queries.len());
    for query in queries {
        let pattern = query.trim();
        if pattern.is_empty() {
            return Err(anyhow!("query is empty"));
        }
        regexes.push(RegexBuilder::new(pattern).multi_line(true).crlf(true).build().map_err(|error| anyhow!("invalid regex {}: {}", pattern, error))?);
    }
    let limit = max_matches.unwrap_or(DEFAULT_MAX_MATCHES).clamp(1, HARD_MAX_MATCHES);
    let glob_regex = match glob.map(str::trim).filter(|item| !item.is_empty()) {
        Some(pattern) => Some(glob_to_regex(pattern)?),
        None => None,
    };
    let found: Vec<AtomicUsize> = (0..regexes.len()).map(|_| AtomicUsize::new(0)).collect();
    let saturated = |found: &[AtomicUsize]| found.iter().all(|counter| counter.load(Ordering::Relaxed) >= limit);

    if root.is_file() {
        let content = fs::read_to_string(root).map_err(|error| anyhow!("read {} failed: {}", root.display(), error))?;
        let mut grouped = Vec::with_capacity(regexes.len());
        for (index, regex) in regexes.iter().enumerate() {
            let mut hits = collect_line_hits(root, &content, regex, limit, limit, &found[index]);
            hits.truncate(limit);
            grouped.push(hits);
        }
        return Ok(grouped);
    }

    let permit = walk_permit();
    let grouped: Mutex<Vec<Vec<SearchHit>>> = Mutex::new(vec![Vec::new(); regexes.len()]);
    let failure = Mutex::new(None);
    WalkBuilder::new(root).hidden(false).git_ignore(true).git_global(true).git_exclude(true).threads(permit.threads).build_parallel().run(|| {
        Box::new(|entry| {
            if saturated(&found) {
                return WalkState::Quit;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    failure.lock().get_or_insert(error.to_string());
                    return WalkState::Quit;
                }
            };
            if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                return WalkState::Continue;
            }
            let path = entry.path();
            if let Some(glob_regex) = glob_regex.as_ref() {
                let relative = path.strip_prefix(root).unwrap_or(path);
                let rel = relative.to_string_lossy().replace('\\', "/");
                if !glob_regex.is_match(&rel) {
                    return WalkState::Continue;
                }
            }
            let Ok(meta) = entry.metadata() else {
                return WalkState::Continue;
            };
            if meta.len() > MAX_FILE_BYTES {
                return WalkState::Continue;
            }
            let Ok(content) = fs::read_to_string(path) else {
                return WalkState::Continue;
            };
            let mut local: Vec<(usize, Vec<SearchHit>)> = Vec::new();
            for (index, regex) in regexes.iter().enumerate() {
                if found[index].load(Ordering::Relaxed) >= limit {
                    continue;
                }
                let matched = collect_line_hits(path, &content, regex, limit, MAX_HITS_PER_FILE, &found[index]);
                if !matched.is_empty() {
                    local.push((index, matched));
                }
            }
            if !local.is_empty() {
                let mut guard = grouped.lock();
                for (index, matched) in local {
                    guard[index].extend(matched);
                }
            }
            if saturated(&found) {
                return WalkState::Quit;
            }
            WalkState::Continue
        })
    });
    if let Some(error) = failure.into_inner() {
        return Err(anyhow!(error));
    }
    let mut grouped = grouped.into_inner();
    for hits in grouped.iter_mut() {
        hits.sort_by(|left, right| left.path.cmp(&right.path).then_with(|| left.line.cmp(&right.line)));
        hits.truncate(limit);
    }
    Ok(grouped)
}

fn collect_line_hits(path: &Path, content: &str, regex: &Regex, limit: usize, per_file: usize, found: &AtomicUsize) -> Vec<SearchHit> {
    let mut hits = Vec::new();
    let mut line = 1usize;
    let mut line_start = 0usize;
    let mut cursor = 0usize;
    for matched in regex.find_iter(content) {
        if hits.len() >= per_file {
            break;
        }
        let start = matched.start();
        if start < cursor {
            continue;
        }
        for (offset, ch) in content[cursor..start].char_indices() {
            if ch == '\n' {
                line += 1;
                line_start = cursor + offset + 1;
            }
        }
        let line_end = content[line_start..].find('\n').map(|pos| line_start + pos).unwrap_or(content.len());
        let (before, after) = context_around(content, line_start, line_end, SEARCH_CONTEXT_LINES);
        hits.push(SearchHit { path: path.to_path_buf(), line, text: content[line_start..line_end].to_string(), before, after });
        cursor = line_end;
        if found.fetch_add(1, Ordering::Relaxed) + 1 >= limit {
            break;
        }
    }
    hits
}

fn context_around(content: &str, line_start: usize, line_end: usize, count: usize) -> (Vec<String>, Vec<String>) {
    let clean = |line: &str| line.trim_end_matches('\r').to_string();
    let mut before: Vec<String> = match content[..line_start].strip_suffix('\n') {
        Some(head) => head.rsplit('\n').take(count).map(clean).collect(),
        None => Vec::new(),
    };
    before.reverse();
    let after: Vec<String> = match content[line_end..].strip_prefix('\n') {
        Some(tail) => tail.split('\n').take(count).map(clean).collect(),
        None => Vec::new(),
    };
    let mut after = after;
    while after.last().is_some_and(|line| line.is_empty()) {
        after.pop();
    }
    (before, after)
}

pub const DEFAULT_INDEX_IGNORE: &[&str] = &[
    ".git/",
    ".svn/",
    ".hg/",
    ".bzr/",
    "_darcs/",
    ".idea/",
    ".vs/",
    ".vscode/",
    ".fleet/",
    ".history/",
    ".DS_Store",
    "node_modules/",
    "bower_components/",
    "jspm_packages/",
    ".pnpm-store/",
    ".yarn/",
    ".npm/",
    "dist/",
    "build/",
    "out/",
    ".next/",
    ".nuxt/",
    ".output/",
    ".svelte-kit/",
    ".astro/",
    ".docusaurus/",
    ".angular/",
    ".turbo/",
    ".parcel-cache/",
    ".cache/",
    ".vercel/",
    ".netlify/",
    "coverage/",
    ".nyc_output/",
    "storybook-static/",
    "*.min.js",
    "*.min.mjs",
    "*.min.css",
    "*.bundle.js",
    "*.chunk.js",
    "*.map",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "npm-shrinkwrap.json",
    "target/",
    "vendor/",
    "Cargo.lock",
    "__pycache__/",
    ".venv/",
    "venv/",
    "virtualenv/",
    "site-packages/",
    ".pytest_cache/",
    ".mypy_cache/",
    ".ruff_cache/",
    ".tox/",
    ".nox/",
    ".hypothesis/",
    ".ipynb_checkpoints/",
    "*.egg-info/",
    ".eggs/",
    "poetry.lock",
    "Pipfile.lock",
    "uv.lock",
    "bin/",
    "obj/",
    ".gradle/",
    ".mvn/",
    ".settings/",
    ".metals/",
    ".bloop/",
    "cmake-build-*/",
    "CMakeFiles/",
    "_build/",
    ".ccls-cache/",
    ".clangd/",
    "compile_commands.json",
    "_deps/",
    "packages/",
    ".nuget/",
    "packages.lock.json",
    "*.Designer.cs",
    "composer.lock",
    ".bundle/",
    "Gemfile.lock",
    "go.sum",
    "Pods/",
    "Carthage/",
    "DerivedData/",
    "*.xcodeproj/",
    "*.xcworkspace/",
    ".dart_tool/",
    ".pub-cache/",
    ".flutter-plugins*",
    ".cxx/",
    "elm-stuff/",
    "deps/",
    ".stack-work/",
    "dist-newstyle/",
    ".terraform/",
    ".serverless/",
    "*.pb.go",
    "*_pb2.py",
    "*_pb2_grpc.py",
    "*.g.dart",
    "*.freezed.dart",
    "*.generated.*",
    "*_generated.*",
    "generated/",
    "gen/",
    ".termini-index/",
];

pub const SECRET_INDEX_IGNORE: &[&str] =
    &[".env", ".env.*", "*.pem", "*.key", "*.p12", "*.pfx", "*.jks", "*.keystore", "id_rsa*", "id_ed25519*", "credentials.json", "service-account*.json", "*.tfstate", "*.tfstate.backup", "secrets.*"];

pub const TEST_INDEX_IGNORE: &[&str] = &["tests/", "test/", "__tests__/", "__snapshots__/", "testdata/", "fixtures/", "*_test.*", "*.test.*", "*_tests.*", "*.spec.*", "*.snap"];

const CUSTOM_IGNORE_FILE: &str = ".terminiignore";

fn build_index_ignore(root: &Path, extra: &[String], include_tests: bool) -> Result<Gitignore> {
    let mut builder = GitignoreBuilder::new(root);
    let mut push = |line: &str| -> Result<()> { builder.add_line(None, line).map(|_| ()).map_err(|error| anyhow!("invalid ignore rule {}: {}", line, error)) };
    for line in DEFAULT_INDEX_IGNORE {
        push(line)?;
    }
    for line in SECRET_INDEX_IGNORE {
        push(line)?;
    }
    if !include_tests {
        for line in TEST_INDEX_IGNORE {
            push(line)?;
        }
    }
    for line in extra {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        push(trimmed)?;
    }
    builder.build().map_err(|error| anyhow!("build ignore failed: {}", error))
}

#[derive(Debug, Clone)]
pub struct IndexProbe {
    pub files: usize,
    pub bytes: u64,
    pub exceeded: bool,
}

pub fn probe_index_size(root: &Path, extra_ignore: &[String], include_tests: bool, max_bytes: u64) -> Result<IndexProbe> {
    if root.is_file() {
        let bytes = fs::metadata(root).map(|meta| meta.len()).unwrap_or(0);
        return Ok(IndexProbe { files: 1, bytes, exceeded: bytes > max_bytes });
    }
    let matcher = build_index_ignore(root, extra_ignore, include_tests)?;
    let permit = walk_permit();
    let files = AtomicUsize::new(0);
    let bytes = AtomicU64::new(0);
    let exceeded = AtomicBool::new(false);
    WalkBuilder::new(root).hidden(false).git_ignore(true).git_global(true).git_exclude(true).add_custom_ignore_filename(CUSTOM_IGNORE_FILE).threads(permit.threads).build_parallel().run(|| {
        Box::new(|entry| {
            if exceeded.load(Ordering::Relaxed) {
                return WalkState::Quit;
            }
            let Ok(entry) = entry else {
                return WalkState::Continue;
            };
            let path = entry.path();
            let is_dir = entry.file_type().is_some_and(|kind| kind.is_dir());
            if matcher.matched(path, is_dir).is_ignore() {
                if is_dir {
                    return WalkState::Skip;
                }
                return WalkState::Continue;
            }
            if is_dir {
                return WalkState::Continue;
            }
            if index_language_for_path(path).is_none() {
                return WalkState::Continue;
            }
            let Ok(meta) = entry.metadata() else {
                return WalkState::Continue;
            };
            if meta.len() > MAX_FILE_BYTES {
                return WalkState::Continue;
            }
            files.fetch_add(1, Ordering::Relaxed);
            let total = bytes.fetch_add(meta.len(), Ordering::Relaxed) + meta.len();
            if total > max_bytes {
                exceeded.store(true, Ordering::Relaxed);
                return WalkState::Quit;
            }
            WalkState::Continue
        })
    });
    Ok(IndexProbe { files: files.load(Ordering::Relaxed), bytes: bytes.load(Ordering::Relaxed), exceeded: exceeded.load(Ordering::Relaxed) })
}

pub fn walk_index_files(root: &Path, extra_ignore: &[String], include_tests: bool) -> Result<Vec<PathBuf>> {
    if root.is_file() {
        return Ok(vec![root.to_path_buf()]);
    }
    let matcher = build_index_ignore(root, extra_ignore, include_tests)?;
    let permit = walk_permit();
    let files = Mutex::new(Vec::new());
    let failure = Mutex::new(None);
    WalkBuilder::new(root).hidden(false).git_ignore(true).git_global(true).git_exclude(true).add_custom_ignore_filename(CUSTOM_IGNORE_FILE).threads(permit.threads).build_parallel().run(|| {
        Box::new(|entry| {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    failure.lock().get_or_insert(error.to_string());
                    return WalkState::Quit;
                }
            };
            let path = entry.path();
            let is_dir = entry.file_type().is_some_and(|kind| kind.is_dir());
            if matcher.matched(path, is_dir).is_ignore() {
                if is_dir {
                    return WalkState::Skip;
                }
                return WalkState::Continue;
            }
            if is_dir {
                return WalkState::Continue;
            }
            if !entry.file_type().is_some_and(|kind| kind.is_file()) {
                return WalkState::Continue;
            }
            if index_language_for_path(path).is_none() {
                return WalkState::Continue;
            }
            let Ok(meta) = entry.metadata() else {
                return WalkState::Continue;
            };
            if meta.len() > MAX_FILE_BYTES {
                return WalkState::Continue;
            }
            files.lock().push(path.to_path_buf());
            WalkState::Continue
        })
    });
    if let Some(error) = failure.into_inner() {
        return Err(anyhow!(error));
    }
    let mut files = files.into_inner();
    files.sort();
    Ok(files)
}

pub fn index_language_for_path(path: &Path) -> Option<&'static str> {
    if let Some(language) = language_for_path(path) {
        return Some(language);
    }
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "md" | "markdown" | "mdx" => Some("markdown"),
        "vue" => Some("vue"),
        "svelte" => Some("svelte"),
        "cs" => Some("csharp"),
        "kt" | "kts" => Some("kotlin"),
        "swift" => Some("swift"),
        "dart" => Some("dart"),
        "scala" | "sc" => Some("scala"),
        "ex" | "exs" => Some("elixir"),
        "erl" | "hrl" => Some("erlang"),
        "hs" => Some("haskell"),
        "sql" => Some("sql"),
        "proto" => Some("proto"),
        "graphql" | "gql" => Some("graphql"),
        "zig" => Some("zig"),
        "nim" => Some("nim"),
        "groovy" | "gradle" => Some("groovy"),
        "r" => Some("r"),
        "jl" => Some("julia"),
        "m" | "mm" => Some("objc"),
        "ini" | "cfg" | "conf" | "properties" | "env" => Some("config"),
        "txt" | "rst" | "adoc" => Some("text"),
        _ => None,
    }
}

pub fn language_for_path(path: &Path) -> Option<&'static str> {
    let file_name = path.file_name()?.to_str()?.to_ascii_lowercase();
    match file_name.as_str() {
        "makefile" | "gnumakefile" => return Some("make"),
        _ => {}
    }
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "rs" => Some("rust"),
        "ts" => Some("typescript"),
        "tsx" => Some("tsx"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        "py" | "pyw" => Some("python"),
        "go" => Some("go"),
        "sh" | "bash" | "zsh" | "ksh" => Some("bash"),
        "java" => Some("java"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" | "hh" | "cu" | "cuh" => Some("cpp"),
        "yml" | "yaml" => Some("yaml"),
        "toml" => Some("toml"),
        "rb" | "rake" => Some("ruby"),
        "lua" => Some("lua"),
        "php" => Some("php"),
        "ps1" | "psm1" | "psd1" => Some("powershell"),
        "json" | "jsonc" => Some("json"),
        "tf" | "hcl" | "tfvars" => Some("hcl"),
        "mk" => Some("make"),
        "bat" | "cmd" => Some("bash"),
        _ => None,
    }
}

pub fn is_data_language(lang: &str) -> bool {
    matches!(lang, "json" | "yaml" | "toml")
}

fn glob_to_regex(glob: &str) -> Result<Regex> {
    let mut out = String::from("(?i)^");
    let mut in_brace = false;
    for ch in glob.chars() {
        match ch {
            '*' => out.push_str(".*"),
            '?' => out.push('.'),
            '{' if !in_brace => {
                in_brace = true;
                out.push_str("(?:");
            }
            '}' if in_brace => {
                in_brace = false;
                out.push(')');
            }
            ',' if in_brace => out.push('|'),
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            other => out.push(other),
        }
    }
    if in_brace {
        return Err(anyhow!("invalid glob: unclosed '{{' in {}", glob));
    }
    out.push('$');
    Regex::new(&out).map_err(|error| anyhow!("invalid glob: {}", error))
}
