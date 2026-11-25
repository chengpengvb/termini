use std::path::Path;

const AGENTS_MD_NAME: &str = "AGENTS.md";
const AGENTS_MD_MAX_BYTES: usize = 32 * 1024;

pub async fn discover_agents_md(file_path: &Path) -> Option<(String, String)> {
    let mut current = file_path.parent();
    while let Some(dir) = current {
        let candidate = dir.join(AGENTS_MD_NAME);
        if let Ok(raw) = tokio::fs::read_to_string(&candidate).await {
            return Some((candidate.display().to_string(), clamp_bytes(raw)));
        }
        current = dir.parent();
    }
    None
}

fn clamp_bytes(raw: String) -> String {
    if raw.len() <= AGENTS_MD_MAX_BYTES {
        return raw;
    }
    let mut end = AGENTS_MD_MAX_BYTES;
    while end > 0 && !raw.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n…（内容过长已截断）", &raw[..end])
}
