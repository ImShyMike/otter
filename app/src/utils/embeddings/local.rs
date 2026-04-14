use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use tracing::debug;

pub const MODEL_NAME: &str = "snowflake/snowflake-arctic-embed-m-quantized";

pub fn get_embeddings(texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
    debug!(
        count = texts.len(),
        model = MODEL_NAME,
        "generating local embeddings"
    );

    let mut model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::SnowflakeArcticEmbedMQ).with_show_download_progress(true),
    )?;
    let all_embeddings = model.embed(texts, None)?;

    debug!(count = all_embeddings.len(), "local embeddings complete");
    Ok(all_embeddings)
}
