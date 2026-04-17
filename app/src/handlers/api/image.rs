use axum::Json;
use axum::extract::{Path, State};
use axum::response::Redirect;
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Serialize, ToSchema)]
pub struct ImageResponse {
    screenshot_url: Option<String>,
}

#[utoipa::path(
    get,
    path = "/image/{id}",
    params(
        ("id" = String, Path, description = "Project ID or Airtable ID"),
    ),
    responses(
        (status = 200, description = "Image URL", body = ImageResponse),
        (status = 404, description = "Not found"),
    )
)]
#[instrument(skip(state))]
pub async fn image(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<ImageResponse>, AppError> {
    let row = get_image_url(&state, &id).await?;

    Ok(Json(ImageResponse {
        screenshot_url: row.as_deref().map(|s| s.to_string()),
    }))
}

#[utoipa::path(
    get,
    path = "/image/{id}/r",
    params(
        ("id" = String, Path, description = "Project ID or Airtable ID"),
    ),
    responses(
        (status = 302, description = "Redirect to image URL"),
        (status = 404, description = "Not found"),
    )
)]
#[instrument(skip(state))]
pub async fn image_redirect(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Redirect, AppError> {
    let url = get_image_url(&state, &id).await?;

    if let Some(url) = url {
        Ok(Redirect::to(&url))
    } else {
        Err(AppError::not_found(format!(
            "No image found for id: {}",
            id
        )))
    }
}

async fn get_image_url(state: &AppState, id: &str) -> Result<Option<String>, AppError> {
    let int_id = id.parse::<i32>().ok();

    let url = if let Some(int_id) = int_id {
        sqlx::query_scalar!("SELECT screenshot_url FROM projects WHERE id = $1", int_id)
            .fetch_one(&state.pg)
            .await
    } else {
        sqlx::query_scalar!(
            "SELECT screenshot_url FROM projects WHERE airtable_id = $1",
            id
        )
        .fetch_one(&state.pg)
        .await
    };

    if let Err(sqlx::Error::RowNotFound) = url {
        return Ok(None);
    } else if let Err(e) = url {
        return Err(e.into());
    }

    Ok(url?)
}
