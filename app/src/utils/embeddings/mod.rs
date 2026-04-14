mod api;
mod local;

use std::env;

use tracing::{debug, info, warn};

fn api_configured() -> bool {
    env::var("AI_API_URL").is_ok()
        && env::var("AI_API_KEY").is_ok()
        && env::var("AI_API_MODEL").is_ok()
}

/// Get embeddings (prioritizing API and falling back to a local model)
pub async fn get_embeddings(texts: &[String]) -> anyhow::Result<(String, Vec<Vec<f32>>)> {
    if !api_configured() {
        debug!("api env vars not set, using local embeddings");
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
