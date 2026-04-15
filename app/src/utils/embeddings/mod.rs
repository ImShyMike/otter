mod api;
pub mod local;

use std::env;

use deadpool_redis::Pool;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tracing::{debug, info, warn};

const CACHE_TTL_SECONDS: usize = 60 * 60 * 24; // 24 hours

#[derive(Serialize, Deserialize)]
struct CachedEmbedding {
    model: String,
    embeddings: Vec<Vec<f32>>,
}

fn api_configured() -> bool {
    env::var("AI_API_URL").is_ok()
        && env::var("AI_API_KEY").is_ok()
        && env::var("AI_API_MODEL").is_ok()
}

fn hash_text(text: &str) -> String {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    format!("embedding:{:x}", hasher.finish())
}

async fn get_from_cache(redis: &Pool, text: &str) -> Option<(String, Vec<f32>)> {
    let key = hash_text(text);
    let mut conn = redis.get().await.ok()?;
    let cached: Option<String> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut *conn)
        .await
        .ok()?;

    if let Some(cached) = cached
        && let Ok(CachedEmbedding { model, embeddings }) = serde_json::from_str(&cached)
    {
        debug!(key, "cache hit for embedding");
        return embeddings.into_iter().next().map(|e| (model, e));
    }
    None
}

async fn store_in_cache(
    redis: &Pool,
    text: &str,
    model: &str,
    embedding: &[f32],
) -> anyhow::Result<()> {
    let key = hash_text(text);
    let cached = CachedEmbedding {
        model: model.to_string(),
        embeddings: vec![embedding.to_vec()],
    };
    let json = serde_json::to_string(&cached)?;

    let mut conn = redis.get().await?;
    redis::cmd("SETEX")
        .arg(&key)
        .arg(CACHE_TTL_SECONDS)
        .arg(&json)
        .query_async::<()>(&mut *conn)
        .await?;
    debug!(key, "cached embedding");
    Ok(())
}

/// Get embeddings (prioritizing API and falling back to a local model) with caching
pub async fn get_embeddings_with_cache(
    texts: &[String],
    redis: &Pool,
    local_only: bool,
) -> anyhow::Result<(String, Vec<Vec<f32>>)> {
    let mut cached_embeddings: Vec<Option<(String, Vec<f32>)>> = vec![None; texts.len()];
    let mut uncached_indices = Vec::new();

    for (i, text) in texts.iter().enumerate() {
        if let Some(cached) = get_from_cache(redis, text).await {
            cached_embeddings[i] = Some(cached);
        } else {
            uncached_indices.push(i);
        }
    }

    if uncached_indices.is_empty() {
        debug!(count = texts.len(), "all embeddings served from cache");
        let model = cached_embeddings[0].as_ref().unwrap().0.clone();
        let embeddings = cached_embeddings
            .iter()
            .map(|c| c.as_ref().unwrap().1.clone())
            .collect();
        return Ok((model, embeddings));
    }

    let uncached_texts: Vec<String> = uncached_indices.iter().map(|&i| texts[i].clone()).collect();

    let (model, uncached_embeddings) = if !api_configured() {
        debug!("api env vars not set, using local embeddings");
        run_local(&uncached_texts).await?
    } else if local_only {
        run_local(&uncached_texts).await?
    } else {
        match api::get_embeddings(&uncached_texts).await {
            Ok(result) => result,
            Err(e) => {
                warn!(error = %e, "api embeddings failed, falling back to local");
                run_local(&uncached_texts).await?
            }
        }
    };

    info!(
        count = uncached_embeddings.len(),
        model, "generated embeddings"
    );
    for (idx, (text, embedding)) in uncached_indices
        .iter()
        .zip(uncached_texts.iter().zip(uncached_embeddings.iter()))
    {
        let _ = store_in_cache(redis, text, &model, embedding).await;
        cached_embeddings[*idx] = Some((model.clone(), embedding.clone()));
    }
    let embeddings = cached_embeddings
        .iter()
        .map(|c| c.as_ref().unwrap().1.clone())
        .collect();
    Ok((model, embeddings))
}

/// Get embeddings (prioritizing API and falling back to a local model)
pub async fn get_embeddings(
    texts: &[String],
    local_only: bool,
) -> anyhow::Result<(String, Vec<Vec<f32>>)> {
    if !api_configured() {
        debug!("api env vars not set, using local embeddings");
        return run_local(texts).await;
    } else if local_only {
        return run_local(texts).await;
    }

    match api::get_embeddings(texts).await {
        Ok((model, embeddings)) => {
            info!(
                count = embeddings.len(),
                model, "generated embeddings via api"
            );
            Ok((model, embeddings))
        }
        Err(e) => {
            warn!(error = %e, "api embeddings failed, falling back to local");
            run_local(texts).await
        }
    }
}

async fn run_local(texts: &[String]) -> anyhow::Result<(String, Vec<Vec<f32>>)> {
    let texts = texts.to_vec();
    let embeddings = tokio::task::spawn_blocking(move || local::get_embeddings(&texts)).await??;
    info!(
        count = embeddings.len(),
        model = local::MODEL_NAME,
        "generated embeddings via local"
    );
    Ok((local::MODEL_NAME.to_string(), embeddings))
}
