use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use tracing::debug;

pub const MODEL_NAME: &str = "intfloat/multilingual-e5-large";

pub fn get_embeddings(texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
    debug!(
        count = texts.len(),
        model = MODEL_NAME,
        "generating local embeddings"
    );

    let mut model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::MultilingualE5Large).with_show_download_progress(true),
    )?;
    let all_embeddings = model.embed(texts, None)?;

    debug!(count = all_embeddings.len(), "local embeddings complete");
    Ok(all_embeddings)
}
