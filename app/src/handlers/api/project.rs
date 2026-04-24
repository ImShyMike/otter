use axum::Json;
use axum::extract::{Path, State};
use tracing::instrument;

use crate::error::AppError;
use crate::handlers::api::ProjectItem;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/project/{id}",
    params(
        ("id" = String, Path, description = "Project ID or Airtable ID"),
    ),
    responses(
        (status = 200, description = "Project information", body = ProjectItem),
        (status = 404, description = "Not found"),
    )
)]
#[instrument(skip(state))]
pub async fn project_info(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ProjectItem>, AppError> {
    let project = if let Ok(project_id) = id.parse::<i32>() {
        sqlx::query_as!(
            ProjectItem,
            "SELECT id, airtable_id, ysws, approved_at, code_url, country, demo_url, description, github_username, hours, true_hours, has_media AS \"has_media!\", github_stars, display_name, archived_demo, archived_repo, inferred_repo, inferred_username FROM projects WHERE deleted_at IS NULL AND id = $1 LIMIT 1",
            project_id
        )
        .fetch_optional(&state.pg)
        .await?
    } else {
        sqlx::query_as!(
            ProjectItem,
            "SELECT id, airtable_id, ysws, approved_at, code_url, country, demo_url, description, github_username, hours, true_hours, has_media AS \"has_media!\", github_stars, display_name, archived_demo, archived_repo, inferred_repo, inferred_username FROM projects WHERE deleted_at IS NULL AND airtable_id = $1 LIMIT 1",
            id
        )
        .fetch_optional(&state.pg)
        .await?
    }
    .ok_or_else(|| AppError::not_found("project not found"))?;

    Ok(Json(project))
}
