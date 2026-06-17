use axum::Json;
use axum::extract::State;
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize, ts_rs::TS, ToSchema)]
pub struct YSWSProgramsResponse(Vec<String>);

#[derive(Serialize, ts_rs::TS, ToSchema)]
pub struct YSWSProgramDetailsResponse {
    name: String,
    total_projects: i64,
    total_hours: f64,
}

#[utoipa_ts::path(
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

#[utoipa_ts::path(
    get,
    path = "/ysws/list/details",
    responses(
        (status = 200, description = "Detailed list of YSWS program names", body = Vec<YSWSProgramDetailsResponse>),
    )
)]
#[instrument(skip(state))]
pub async fn ysws_program_list_details(
    State(state): State<AppState>,
) -> Result<Json<Vec<YSWSProgramDetailsResponse>>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT
            ysws AS ysws,
            COUNT(*) AS total_projects,
            SUM(COALESCE(true_hours, hours, 0)) AS total_hours
        FROM projects
        WHERE ysws IS NOT NULL
        GROUP BY ysws
        "#
    )
    .fetch_all(&state.pg)
    .await?;

    let response = rows
        .into_iter()
        .map(|row| YSWSProgramDetailsResponse {
            name: row.ysws,
            total_projects: row.total_projects.unwrap_or(0),
            total_hours: row.total_hours.unwrap_or(0.0),
        })
        .collect();

    Ok(Json(response))
}
