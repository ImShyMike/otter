pub mod image;
pub mod query;
pub mod search;
pub mod ysws_programs;

use crate::utils::embeddings;
use std::sync::OnceLock;
use tracing::warn;

static LOCAL_ONLY: OnceLock<bool> = OnceLock::new();

pub fn local_only() -> bool {
    *LOCAL_ONLY.get_or_init(|| {
        let force_local = std::env::var("API_FORCE_LOCAL")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        let api_ai_model = std::env::var("AI_API_MODEL").ok();
        let local_model = embeddings::local::MODEL_NAME;
        if force_local {
            if api_ai_model.as_deref() != Some(local_model) {
                warn!("API_FORCE_LOCAL is set but AI_API_MODEL does not match local model, ignoring...");
                false
            } else {
                true
            }
        } else {
            false
        }
    })
}
