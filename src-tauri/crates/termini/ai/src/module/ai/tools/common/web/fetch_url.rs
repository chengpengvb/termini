use std::time::Duration;
use crate::module::ai::tools::common::web::html_text;
use crate::module::ai::tools::ai_tool_error::AiToolError;

const REQUEST_TIMEOUT_SECS: u64 = 20;
const WINDOW_DEFAULT_LINES: usize = 200;

pub fn resolve_url(raw: &str) -> Result<url::Url, AiToolError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AiToolError::Execution("url is empty".to_string()));
    }
    let mut parsed = url::Url::parse(trimmed).map_err(|error| AiToolError::Execution(format!("invalid url: {}", error)))?;
    if parsed.scheme() == "http" {
        parsed.set_scheme("https").map_err(|_| AiToolError::Execution("failed to upgrade http to https".to_string()))?;
    }
    if parsed.scheme() != "https" {
        return Err(AiToolError::Execution(format!("only https urls are allowed: {}", trimmed)));
    }
    let host = match parsed.host_str() {
        Some(value) if !value.is_empty() => value,
        _ => {
            return Err(AiToolError::Execution("url host is empty".to_string()));
        }
    };
    if is_blocked_host(host) {
        return Err(AiToolError::Execution(format!("url host is not allowed: {}", host)));
    }
    Ok(parsed)
}

fn is_blocked_host(host: &str) -> bool {
    let lower = host.to_ascii_lowercase();
    if lower == "localhost" || lower.ends_with(".localhost") || lower.ends_with(".local") {
        return true;
    }
    if let Ok(ip) = lower.parse::<std::net::IpAddr>() {
        return match ip {
            std::net::IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local() || v4.is_unspecified() || v4.octets()[0] == 169 && v4.octets()[1] == 254,
            std::net::IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified() || (v6.segments()[0] & 0xffc0) == 0xfe80,
        };
    }
    false
}

pub fn is_trusted_url(url: &url::Url, trusted_hosts: &[String]) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    let lower = host.to_ascii_lowercase();
    trusted_hosts.iter().any(|trusted| lower == *trusted || lower.strip_suffix(trusted.as_str()).is_some_and(|prefix| prefix.ends_with('.')))
}

fn describe_failed_status(status: u16) -> String {
    if matches!(status, 401 | 403 | 429 | 503) {
        return format!("http status {}: this site has anti-bot or login protection and plain HTTP cannot fetch the content. Do not retry other URLs on the same site; answer from the web_search summary instead, or tell the user the site requires a browser", status);
    }
    format!("http status {}", status)
}

pub async fn fetch_url(url: url::Url, raw: bool) -> Result<String, AiToolError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|error| AiToolError::Execution(error.to_string()))?;
    let response = client
        .get(url)
        .header(reqwest::header::ACCEPT, "text/markdown, text/plain, text/html;q=0.9, */*;q=0.8")
        .header(reqwest::header::USER_AGENT, "TermixWebFetch/1.0")
        .send()
        .await
        .map_err(|error| AiToolError::Execution(error.to_string()))?;
    let status = response.status();
    if !status.is_success() {
        return Err(AiToolError::Execution(describe_failed_status(status.as_u16())));
    }
    let html_type = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|value| value.to_str().ok()).map(|value| value.to_ascii_lowercase().contains("text/html")).unwrap_or(false);
    let body = response.text().await.map_err(|error| AiToolError::Execution(error.to_string()))?;
    if raw || !(html_type || html_text::looks_like_html(&body)) {
        return Ok(body);
    }
    Ok(html_text::html_to_text(&body))
}

pub fn present(body: String, offset: Option<usize>, limit: Option<usize>) -> Result<String, AiToolError> {
    if offset.is_none() && limit.is_none() {
        return Ok(body);
    }
    let lines: Vec<&str> = body.lines().collect();
    let total = lines.len();
    let start = offset.unwrap_or(1).max(1) - 1;
    if start >= total {
        return Err(AiToolError::Execution(format!("offset {} is beyond the last line ({} lines total)", start + 1, total)));
    }
    let take = limit.unwrap_or(WINDOW_DEFAULT_LINES).max(1);
    let end = (start + take).min(total);
    let mut out = format!("[web_fetch] showing lines {}-{} of {}\n", start + 1, end, total);
    for (index, line) in lines[start..end].iter().enumerate() {
        out.push_str(&format!("{}→{}\n", start + index + 1, line));
    }
    if end < total {
        out.push_str(&format!("… {} more lines, continue with offset={}", total - end, end + 1));
    }
    Ok(out)
}
