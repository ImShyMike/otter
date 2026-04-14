use std::{env, sync::OnceLock, time::Duration};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

const BATCH_SIZE: usize = 128;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

struct ApiConfig {
    client: Client,
    url: String,
    key: String,
    model: String,
}

static CONFIG: OnceLock<Result<ApiConfig, String>> = OnceLock::new();

fn config() -> anyhow::Result<&'static ApiConfig> {
    CONFIG
        .get_or_init(|| {
            (|| -> anyhow::Result<ApiConfig> {
                let url = env::var("AI_API_URL")?;
                let key = env::var("AI_API_KEY")?;
                let model = env::var("AI_API_MODEL")?;

                let api_host = reqwest::Url::parse(&url)?
                    .host_str()
                    .unwrap_or_default()
                    .to_string();

                let client = Client::builder()
                    .timeout(REQUEST_TIMEOUT)
                    .retry(
                        reqwest::retry::for_host(api_host)
                            .max_retries_per_request(3)
                            .classify_fn(|req_rep| match req_rep.status() {
                                Some(s) if s.is_server_error() => req_rep.retryable(),
                                None => req_rep.retryable(),
                                _ => req_rep.success(),
                            }),
                    )
                    .build()?;

                Ok(ApiConfig {
                    client,
                    url,
                    key,
                    model,
                })
            })()
            .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(|e| anyhow::anyhow!("{e}"))
}

#[derive(Deserialize)]
struct EmbeddingsResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Serialize)]
struct RequestData {
    input: Vec<String>,
    model: String,
    dimensions: u32,
}

pub async fn get_embeddings(texts: &[String]) -> anyhow::Result<(String, Vec<Vec<f32>>)> {
    let cfg = config()?;
    let mut embeddings = Vec::new();

    for (i, batch) in texts.chunks(BATCH_SIZE).enumerate() {
        debug!(
            batch = i + 1,
            size = batch.len(),
            "requesting api embeddings"
        );

        let response = cfg
            .client
            .post(&cfg.url)
            .header("Authorization", format!("Bearer {}", cfg.key))
            .json(&RequestData {
                input: batch.to_vec(),
                model: cfg.model.clone(),
                dimensions: 768,
            })
            .send()
            .await?
            .error_for_status()?
            .json::<EmbeddingsResponse>()
            .await?;

        for item in response.data {
            embeddings.push(item.embedding);
        }
    }

    debug!(
        count = embeddings.len(),
        model = cfg.model,
        "api embeddings complete"
    );
    Ok((cfg.model.clone(), embeddings))
}
