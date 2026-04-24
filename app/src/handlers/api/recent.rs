use axum::Json;
use axum::extract::{State, Query};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct RecentProjectsQuery {
    limit: Option<i64>,
}

#[derive(Serialize, ToSchema)]
pub struct ProgramResponse {
    id: i32,
    airtable_id: String,
    ysws: String,
    approved_at: Option<i64>,
    code_url: Option<String>,
    country: Option<String>,
    demo_url: Option<String>,
    description: Option<String>,
    github_username: Option<String>,
    hours: Option<i32>,
    true_hours: Option<f64>,
    has_media: bool,
    github_stars: i32,
    display_name: Option<String>,
    archived_demo: Option<String>,
    archived_repo: Option<String>,
    inferred_repo: Option<String>,
    inferred_username: Option<String>,
}

#[utoipa::path(
    get,
    path = "/recent",
    responses(
        (status = 200, description = "Recently indexed projects", body = Vec<ProgramResponse>),
        (status = 404, description = "Not found"),
    )
)]
#[instrument(skip(state))]
pub async fn recent_projects(
    State(state): State<AppState>,
    Query(params): Query<RecentProjectsQuery>,
) -> Result<Json<Vec<ProgramResponse>>, AppError> {
    let limit = params.limit.unwrap_or(20).min(100);

    let projects = sqlx::query_as!(
            ProgramResponse,
            "SELECT id, airtable_id, ysws, EXTRACT(EPOCH FROM approved_at)::bigint AS approved_at, code_url, country, demo_url, description, github_username, hours, true_hours, (media_url IS NOT NULL) AS \"has_media!\", github_stars, display_name, archived_demo, archived_repo, inferred_repo, inferred_github_username AS inferred_username FROM projects WHERE deleted_at IS NULL ORDER BY created_at DESC LIMIT $1",
            limit
        )
        .fetch_all(&state.pg)
        .await?;

    Ok(Json(projects))
}
