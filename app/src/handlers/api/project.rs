use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

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
    path = "/project/{id}",
    params(
        ("id" = String, Path, description = "Project ID or Airtable ID"),
    ),
    responses(
        (status = 200, description = "Project information", body = ProgramResponse),
        (status = 404, description = "Not found"),
    )
)]
#[instrument(skip(state))]
pub async fn project_info(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ProgramResponse>, AppError> {
    let project = if let Ok(project_id) = id.parse::<i32>() {
        sqlx::query_as!(
            ProgramResponse,
            "SELECT id, airtable_id, ysws, EXTRACT(EPOCH FROM approved_at)::bigint AS approved_at, code_url, country, demo_url, description, github_username, hours, true_hours, (media_url IS NOT NULL) AS \"has_media!\", github_stars, display_name, archived_demo, archived_repo, inferred_repo, inferred_github_username AS inferred_username FROM projects WHERE deleted_at IS NULL AND id = $1 LIMIT 1",
            project_id
        )
        .fetch_optional(&state.pg)
        .await?
    } else {
        sqlx::query_as!(
            ProgramResponse,
            "SELECT id, airtable_id, ysws, EXTRACT(EPOCH FROM approved_at)::bigint AS approved_at, code_url, country, demo_url, description, github_username, hours, true_hours, (media_url IS NOT NULL) AS \"has_media!\", github_stars, display_name, archived_demo, archived_repo, inferred_repo, inferred_github_username AS inferred_username FROM projects WHERE deleted_at IS NULL AND airtable_id = $1 LIMIT 1",
            id
        )
        .fetch_optional(&state.pg)
        .await?
    }
    .ok_or_else(|| AppError::not_found("project not found"))?;

    Ok(Json(project))
}
