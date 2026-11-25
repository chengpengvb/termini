use serde_json::json;
use serde_json::Value;
use futures::StreamExt;
use anyhow::{anyhow, Context, Result};
use crate::model::ai_provide::AiProvide;
use std::sync::atomic::{AtomicUsize, Ordering};

const PARAM_ID: &str = "id";
const BATCH_CONCURRENCY: usize = 12;
pub type ProgressFn<'a> = &'a (dyn Fn(usize, usize) + Send + Sync);
pub type BatchSink<'a> = &'a (dyn Fn(usize, &[Vec<f32>]) -> Result<()> + Send + Sync);

pub async fn probe_embedding(provide: &AiProvide) -> Result<()> {
    let url = resolve_embeddings_url(&provide.url);
    let key = provide.key.clone().unwrap_or_default();
    let client = reqwest::Client::new();
    let body = json!({ "model": provide.model, "input": ["ping"] });
    let response = client.post(url).bearer_auth(&key).json(&body).send().await.context("embeddings probe failed")?;
    let status = response.status();
    let text = response.text().await.context("failed to read embeddings probe response")?;
    if !status.is_success() {
        return Err(anyhow!("embeddings returned {}: {}", status.as_u16(), text));
    }
    let value: Value = serde_json::from_str(&text).context("failed to parse embeddings probe response")?;
    let items = value.get("data").and_then(Value::as_array).ok_or_else(|| anyhow!("embeddings response is missing data: {}", text))?;
    if items.is_empty() {
        return Err(anyhow!("embeddings probe returned no vectors"));
    }
    Ok(())
}

pub async fn embed_all(provide: &AiProvide, texts: &[String], batch_size: usize, on_progress: ProgressFn<'_>) -> Result<Vec<Vec<f32>>> {
    let noop = |_start: usize, _vectors: &[Vec<f32>]| Ok(());
    embed_all_with_sink(provide, texts, batch_size, on_progress, &noop).await
}

pub async fn embed_all_with_sink(provide: &AiProvide, texts: &[String], batch_size: usize, on_progress: ProgressFn<'_>, on_batch: BatchSink<'_>) -> Result<Vec<Vec<f32>>> {
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    let provide_id = resolve_provide_id(provide)?;
    let url = resolve_embeddings_url(&provide.url);
    let key = provide.key.clone().ok_or_else(|| anyhow!("ai_provide.key is not set"))?;
    let client = reqwest::Client::new();
    let size = batch_size.max(1);
    let batches: Vec<Vec<String>> = texts.chunks(size).map(|chunk| chunk.to_vec()).collect();
    let total = batches.len();
    let done = AtomicUsize::new(0);
    let results = futures::stream::iter(batches.into_iter().enumerate())
        .map(|(index, batch)| {
            let url = url.clone();
            let key = key.clone();
            let client = client.clone();
            let provide_id = provide_id.clone();
            let done = &done;
            async move {
                let vectors = embed_batch(&client, &url, &key, &provide_id, &batch).await.with_context(|| format!("embed batch {}/{}", index + 1, total))?;
                on_batch(index * size, &vectors).with_context(|| format!("checkpoint batch {}/{}", index + 1, total))?;
                let finished = done.fetch_add(batch.len(), Ordering::Relaxed) + batch.len();
                on_progress(finished, texts.len());
                Ok::<(usize, Vec<Vec<f32>>), anyhow::Error>((index, vectors))
            }
        })
        .buffer_unordered(BATCH_CONCURRENCY)
        .collect::<Vec<_>>()
        .await;
    let mut ordered: Vec<Option<Vec<Vec<f32>>>> = vec![None; total];
    for result in results {
        let (index, vectors) = result?;
        ordered[index] = Some(vectors);
    }
    let mut out = Vec::with_capacity(texts.len());
    for (index, slot) in ordered.into_iter().enumerate() {
        let vectors = slot.ok_or_else(|| anyhow!("embed batch {} has no result", index + 1))?;
        out.extend(vectors);
    }
    if out.len() != texts.len() {
        return Err(anyhow!("embedding count mismatch: expected {} actual {}", texts.len(), out.len()));
    }
    Ok(out)
}

async fn embed_batch(client: &reqwest::Client, url: &str, key: &str, provide_id: &str, batch: &[String]) -> Result<Vec<Vec<f32>>> {
    let body = json!({ PARAM_ID: provide_id, "input": batch });
    let response = client.post(url).bearer_auth(key).json(&body).send().await.context("embeddings request failed")?;
    let status = response.status();
    let text = response.text().await.context("failed to read embeddings response")?;
    if !status.is_success() {
        return Err(anyhow!("embeddings returned {}: {}", status.as_u16(), text));
    }
    let value: Value = serde_json::from_str(&text).context("failed to parse embeddings response")?;
    let items = value.get("data").and_then(Value::as_array).ok_or_else(|| anyhow!("embeddings response is missing data: {}", text))?;
    let mut rows: Vec<(usize, Vec<f32>)> = Vec::with_capacity(items.len());
    for (position, item) in items.iter().enumerate() {
        let index = item.get("index").and_then(Value::as_u64).map(|value| value as usize).unwrap_or(position);
        let embedding = item.get("embedding").and_then(Value::as_array).ok_or_else(|| anyhow!("embeddings response is missing embedding"))?;
        let mut vector = Vec::with_capacity(embedding.len());
        for number in embedding {
            vector.push(number.as_f64().ok_or_else(|| anyhow!("embedding element is not a number"))? as f32);
        }
        rows.push((index, vector));
    }
    rows.sort_by_key(|(index, _)| *index);
    if rows.len() != batch.len() {
        return Err(anyhow!("embeddings count mismatch: expected {} actual {}", batch.len(), rows.len()));
    }
    Ok(rows.into_iter().map(|(_, vector)| vector).collect())
}

fn resolve_provide_id(provide: &AiProvide) -> Result<String> {
    let Some(Value::Object(map)) = &provide.params else {
        return Err(anyhow!("ai_provide.params is missing {}", PARAM_ID));
    };
    let Some(Value::String(id)) = map.get(PARAM_ID) else {
        return Err(anyhow!("ai_provide.params.{} is not a string", PARAM_ID));
    };
    Ok(id.clone())
}

fn resolve_embeddings_url(raw: &str) -> String {
    let mut url = raw.trim().to_string();
    while url.ends_with('/') {
        url.pop();
    }
    if url.ends_with("/embeddings") {
        return url;
    }
    if url.ends_with("/v1") {
        return format!("{}/embeddings", url);
    }
    format!("{}/v1/embeddings", url)
}
