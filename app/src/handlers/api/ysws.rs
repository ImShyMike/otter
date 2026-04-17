use axum::Json;
use axum::extract::{Path, State};
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize, ToSchema)]
pub struct YSWSProgramsResponse(Vec<String>);

#[utoipa::path(
    get,
    path = "/ysws/list",
    responses(
        (status = 200, description = "List of YSWS program names", body = Vec<String>),
    )
)]
#[instrument(skip(state))]
pub async fn ysws_program_list(
    State(state): State<AppState>,
) -> Result<Json<YSWSProgramsResponse>, AppError> {
    let row = sqlx::query_scalar!("SELECT DISTINCT ysws FROM projects")
        .fetch_all(&state.pg)
        .await?;

    Ok(Json(YSWSProgramsResponse(
        row.into_iter().filter_map(Some).collect(),
    )))
}

#[derive(Serialize, ToSchema)]
pub struct YSWSProjectsResponse {
    id: i32,
    airtable_id: Option<String>,
    display_name: Option<String>,
    description: Option<String>,
    ysws: String,
    country: Option<String>,
    code_url: Option<String>,
    demo_url: Option<String>,
}
