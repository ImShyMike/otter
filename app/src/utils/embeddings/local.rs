use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::Mutex;
use tracing::debug;

pub const MODEL_NAME: &str = "intfloat/multilingual-e5-large";

static MODEL: Mutex<Option<TextEmbedding>> = Mutex::new(None);

pub fn get_embeddings(texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
    debug!(
        count = texts.len(),
        model = MODEL_NAME,
        "generating local embeddings"
    );

    let mut model = MODEL
        .lock()
        .map_err(|e| anyhow::anyhow!("local embeddings model lock poisoned: {e}"))?;
    if model.is_none() {
        debug!(model = MODEL_NAME, "loading local embeddings model");
        *model = Some(TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::MultilingualE5Large).with_show_download_progress(true),
        )?);
    }

    let model = model
        .as_mut()
        .ok_or_else(|| anyhow::anyhow!("local embeddings model failed to initialize"))?;
    let all_embeddings = model.embed(texts, None)?;

    debug!(count = all_embeddings.len(), "local embeddings complete");
    Ok(all_embeddings)
}
