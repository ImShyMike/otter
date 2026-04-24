pub mod media;
pub mod project;
pub mod query;
pub mod recent;
pub mod search;
pub mod ysws;

use serde::Serialize;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::utils::embeddings;
use std::sync::OnceLock;
use tracing::warn;

#[derive(Serialize, ToSchema)]
pub struct ProjectItem {
    pub id: i32,
    pub airtable_id: String,
    pub ysws: String,
    pub approved_at: Option<i64>,
    pub code_url: Option<String>,
    pub country: Option<String>,
    pub demo_url: Option<String>,
    pub description: Option<String>,
    pub github_username: Option<String>,
    pub hours: Option<i32>,
    pub true_hours: Option<f64>,
    pub has_media: bool,
    pub github_stars: i32,
    pub display_name: Option<String>,
    pub archived_demo: Option<String>,
    pub archived_repo: Option<String>,
    pub inferred_repo: Option<String>,
    pub inferred_username: Option<String>,
}

pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(search::search))
        .routes(routes!(ysws::ysws_program_list))
        .routes(routes!(query::query))
        .routes(routes!(media::media))
        .routes(routes!(media::media_redirect))
        .routes(routes!(project::project_info))
        .routes(routes!(recent::recent_projects))
}

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
