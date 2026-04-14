use std::{env, time::Duration};

use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::debug;

const BATCH_SIZE: usize = 64;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

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
}

pub async fn get_embeddings(texts: &[String]) -> anyhow::Result<(String, Vec<Vec<f32>>)> {
    let api_url = env::var("AI_API_URL")?;
    let api_key = env::var("AI_API_KEY")?;
    let api_model = env::var("AI_API_MODEL")?;

    let client = Client::builder().timeout(REQUEST_TIMEOUT).build()?;
    let mut embeddings = Vec::new();

    for (i, batch) in texts.chunks(BATCH_SIZE).enumerate() {
        debug!(
            batch = i + 1,
            size = batch.len(),
            "requesting api embeddings"
        );

        let response = client
            .post(&api_url)
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&RequestData {
                input: batch.to_vec(),
                model: api_model.clone(),
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
        model = api_model,
        "api embeddings complete"
    );
    Ok((api_model, embeddings))
}
