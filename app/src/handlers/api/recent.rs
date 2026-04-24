use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;
use tracing::instrument;

use crate::error::AppError;
use crate::handlers::api::ProjectItem;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RecentProjectsQuery {
    limit: Option<i64>,
}

#[utoipa::path(
    get,
    path = "/recent",
    responses(
        (status = 200, description = "Recently indexed projects", body = Vec<ProjectItem>),
        (status = 404, description = "Not found"),
    )
)]
#[instrument(skip(state))]
pub async fn recent_projects(
    State(state): State<AppState>,
    Query(params): Query<RecentProjectsQuery>,
) -> Result<Json<Vec<ProjectItem>>, AppError> {
    let limit = params.limit.unwrap_or(20).min(100);

    let projects = sqlx::query_as!(
            ProjectItem,
            "SELECT id, airtable_id, ysws, approved_at, code_url, country, demo_url, description, github_username, hours, true_hours, has_media AS \"has_media!\", github_stars, display_name, archived_demo, archived_repo, inferred_repo, inferred_username FROM projects WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT $1",
            limit
        )
        .fetch_all(&state.pg)
        .await?;

    Ok(Json(projects))
}
