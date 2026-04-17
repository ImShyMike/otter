use axum::Json;
use axum::extract::State;
use redis::AsyncCommands;
use serde::Serialize;
use tracing::instrument;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ApiResponse {
    value: String,
}

#[instrument(skip(state))]
pub async fn health(State(state): State<AppState>) -> Result<Json<ApiResponse>, AppError> {
    let mut conn = state.redis.get().await?;
    let _: () = conn.set("key", "up").await?;
    let val: String = conn.get("key").await?;

    let row = sqlx::query_scalar!("SELECT 1").fetch_one(&state.pg).await?;

    Ok(Json(ApiResponse {
        value: format!("redis={val}, pg={}", row.unwrap_or(0)),
    }))
}
