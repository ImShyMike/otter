use axum::Json;
use axum::extract::State;
use serde::Serialize;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize, ToSchema)]
pub struct YSWSProgramsResponse(Vec<String>);

#[utoipa::path(
    get,
    path = "/api/ysws_programs",
    responses(
        (status = 200, description = "List of YSWS program names", body = Vec<String>),
    )
)]
pub async fn ysws_programs(
    State(state): State<AppState>,
) -> Result<Json<YSWSProgramsResponse>, AppError> {
    let row = sqlx::query_scalar!("SELECT DISTINCT ysws FROM projects")
        .fetch_all(&state.pg)
        .await?;

    Ok(Json(YSWSProgramsResponse(
        row.into_iter().filter_map(Some).collect(),
    )))
}
