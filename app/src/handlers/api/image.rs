use axum::Json;
use axum::extract::{State, Path};
use axum::response::Redirect;
use serde::Serialize;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize)]
pub struct ApiResponse {
    screenshot_url: Option<String>,
}

pub async fn image(State(state): State<AppState>, Path(id): Path<String>) -> Result<Json<ApiResponse>, AppError> {
    let row = get_image_url(&state, &id).await?;

    Ok(Json(ApiResponse {
        screenshot_url: row.as_deref().map(|s| s.to_string()),
    }))
}

pub async fn image_redirect(State(state): State<AppState>, Path(id): Path<String>) -> Result<Redirect, AppError> {
    let url = get_image_url(&state, &id).await?;

    Ok(Redirect::to(url.as_deref().unwrap_or("/")))
}

async fn get_image_url(state: &AppState, id: &str) -> Result<Option<String>, AppError> {
    let int_id = id.parse::<i32>().ok();

    let url = if let Some(int_id) = int_id {
        sqlx::query_scalar!("SELECT screenshot_url FROM projects WHERE id = $1", int_id)
            .fetch_one(&state.pg)
            .await?
    } else {
        sqlx::query_scalar!("SELECT screenshot_url FROM projects WHERE airtable_id = $1", id)
            .fetch_one(&state.pg)
            .await?
    };

    Ok(url)
}
