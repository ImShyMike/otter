use axum::Json;
use axum::extract::State;

use crate::error::AppError;
use crate::state::AppState;

type ApiResponse = Vec<String>;

pub async fn ysws_programs(State(state): State<AppState>) -> Result<Json<ApiResponse>, AppError> {
    let row = sqlx::query_scalar!("SELECT DISTINCT ysws FROM projects")
        .fetch_all(&state.pg)
        .await?;

    Ok(Json(row))
}
