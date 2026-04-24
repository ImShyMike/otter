use axum::Json;
use axum::extract::{Path, State};
use axum::response::Redirect;
use deadpool_redis::redis::AsyncCommands;
use serde::Serialize;
use tracing::instrument;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

const NULL_MEDIA_URL: &str = "__NULL_MEDIA_URL__";

#[derive(Serialize, ToSchema)]
pub struct MediaResponse {
    url: Option<String>,
}

#[utoipa::path(
    get,
    path = "/media/{id}",
    params(
        ("id" = String, Path, description = "Project ID or Airtable ID"),
    ),
    responses(
        (status = 200, description = "Media URL", body = MediaResponse),
        (status = 404, description = "Not found"),
    )
)]
#[instrument(skip(state))]
pub async fn media(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<MediaResponse>, AppError> {
    let row = get_media_url(&state, &id).await?;

    Ok(Json(MediaResponse {
        url: row.as_deref().map(|s| s.to_string()),
    }))
}

#[utoipa::path(
    get,
    path = "/media/{id}/r",
    params(
        ("id" = String, Path, description = "Project ID or Airtable ID"),
    ),
    responses(
        (status = 303, description = "Redirect to media URL"),
        (status = 404, description = "Not found"),
    )
)]
#[instrument(skip(state))]
pub async fn media_redirect(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Redirect, AppError> {
    let url = get_media_url(&state, &id).await?;

    if let Some(url) = url {
        Ok(Redirect::to(&url))
    } else {
        Err(AppError::not_found(format!(
            "No image found for id: {}",
            id
        )))
    }
}

#[instrument(skip(state))]
async fn get_media_url(state: &AppState, id: &str) -> Result<Option<String>, AppError> {
    let int_id = id.parse::<i32>().ok();
    let cache_key = if let Some(int_id) = int_id {
        format!("media_url:{}", int_id)
    } else {
        format!("media_url:{}", id)
    };
    let mut conn = state.redis.get().await?;

    let url = if let Some(int_id) = int_id {
        if let Ok(Some(cached_url)) = conn.get::<_, Option<String>>(&cache_key).await {
            return Ok(match cached_url.as_str() {
                NULL_MEDIA_URL => None,
                cached_url => Some(cached_url.to_string()),
            });
        }
        sqlx::query_scalar!("SELECT media_url FROM projects WHERE id = $1", int_id)
            .fetch_one(&state.pg)
            .await
    } else {
        if let Ok(Some(cached_url)) = conn.get::<_, Option<String>>(&cache_key).await {
            return Ok(match cached_url.as_str() {
                NULL_MEDIA_URL => None,
                cached_url => Some(cached_url.to_string()),
            });
        }
        sqlx::query_scalar!("SELECT media_url FROM projects WHERE airtable_id = $1", id)
            .fetch_one(&state.pg)
            .await
    };

    let url = match url {
        Ok(url) => url,
        Err(sqlx::Error::RowNotFound) => None,
        Err(e) => return Err(e.into()),
    };

    let cached_url = url.as_deref().unwrap_or(NULL_MEDIA_URL);
    let _: () = conn.set_ex(&cache_key, cached_url, 60 * 60).await?;

    Ok(url)
}
