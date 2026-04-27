use axum::Json;
use axum::extract::{Path, State};
use axum::response::Redirect;
use deadpool_redis::redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::instrument;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

const NULL_MEDIA_SENTINEL: &str = "__NULL_MEDIA__";
const CACHE_TTL_SECS: u64 = 60 * 60;
const MAX_BATCH_IDS: usize = 100;

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct MediaItem {
    pub id: i32,
    pub project_id: i32,
    pub airtable_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,
    pub mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i32>,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_small_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_small_width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_small_height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_large_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_large_width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_large_height: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_full_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_full_width: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_full_height: Option<i32>,
}

#[derive(Serialize, Deserialize, ToSchema, Clone, Debug)]
pub struct SlimMediaItem {
    #[serde(skip_serializing)]
    pub project_id: i32,
    pub mime_type: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_large_url: Option<String>,
}

#[utoipa::path(
    get,
    path = "/media/{id}",
    params(
        ("id" = String, Path, description = "Project ID or Airtable ID"),
    ),
    responses(
        (status = 200, description = "All media items for the project", body = Vec<MediaItem>),
        (status = 404, description = "Not found"),
    )
)]
#[instrument(skip(state))]
pub async fn media(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Vec<MediaItem>>, AppError> {
    let items = get_media_items(&state, &id).await?;
    Ok(Json(items))
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
    let items = get_media_items(&state, &id).await?;

    if let Some(first) = items.first() {
        Ok(Redirect::to(&first.url))
    } else {
        Err(AppError::not_found(format!(
            "No image found for id: {}",
            id
        )))
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MediaBatchRequest {
    pub ids: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct MediaBatchResponse {
    pub media: HashMap<String, Vec<SlimMediaItem>>,
}
#[utoipa::path(
    post,
    path = "/media/batch",
    request_body = MediaBatchRequest,
    responses(
        (status = 200, description = "Media items grouped by requested id", body = MediaBatchResponse),
        (status = 400, description = "Bad request"),
    )
)]
#[instrument(skip(state, body), fields(ids = body.ids.len()))]
pub async fn media_batch(
    State(state): State<AppState>,
    Json(body): Json<MediaBatchRequest>,
) -> Result<Json<MediaBatchResponse>, AppError> {
    if body.ids.is_empty() {
        return Ok(Json(MediaBatchResponse {
            media: HashMap::new(),
        }));
    }
    if body.ids.len() > MAX_BATCH_IDS {
        return Err(AppError::bad_request(format!(
            "Too many ids (max {})",
            MAX_BATCH_IDS
        )));
    }

    let mut conn = state.redis.get().await?;
    let mut media: HashMap<String, Vec<SlimMediaItem>> = HashMap::with_capacity(body.ids.len());
    let mut missing_int_ids: Vec<i32> = Vec::new();
    let mut missing_airtable_ids: Vec<String> = Vec::new();
    let mut int_origin: HashMap<i32, String> = HashMap::new();
    let mut airtable_origin: HashMap<String, String> = HashMap::new();

    for id in &body.ids {
        if media.contains_key(id) {
            continue;
        }
        let cache_key = cache_key_for_slim(id);
        match conn.get::<_, Option<String>>(&cache_key).await {
            Ok(Some(cached)) if cached == NULL_MEDIA_SENTINEL => {
                media.insert(id.clone(), Vec::new());
                continue;
            }
            Ok(Some(cached)) => {
                if let Ok(items) = serde_json::from_str::<Vec<SlimMediaItem>>(&cached) {
                    media.insert(id.clone(), items);
                    continue;
                }
            }
            _ => {}
        }

        if let Ok(int_id) = id.parse::<i32>() {
            missing_int_ids.push(int_id);
            int_origin.insert(int_id, id.clone());
        } else {
            missing_airtable_ids.push(id.clone());
            airtable_origin.insert(id.clone(), id.clone());
        }
    }

    if !missing_int_ids.is_empty() {
        let rows = sqlx::query_as!(
            SlimMediaItem,
            "SELECT project_id, mime_type, url, thumb_large_url \
             FROM media WHERE project_id = ANY($1) ORDER BY project_id, id",
            &missing_int_ids
        )
        .fetch_all(&state.pg)
        .await?;

        let mut grouped: HashMap<i32, Vec<SlimMediaItem>> = HashMap::new();
        for row in rows {
            grouped.entry(row.project_id).or_default().push(row);
        }
        for int_id in &missing_int_ids {
            let items = grouped.remove(int_id).unwrap_or_default();
            cache_slim_items(&mut conn, &cache_key_for_slim(&int_id.to_string()), &items).await?;
            if let Some(origin) = int_origin.remove(int_id) {
                media.insert(origin, items);
            }
        }
    }

    if !missing_airtable_ids.is_empty() {
        let rows = sqlx::query!(
            "SELECT m.project_id, m.mime_type, m.url, m.thumb_large_url, m.thumb_full_url, \
             p.airtable_id AS project_airtable_id \
             FROM media m INNER JOIN projects p ON p.id = m.project_id \
             WHERE p.airtable_id = ANY($1) ORDER BY p.airtable_id, m.id",
            &missing_airtable_ids
        )
        .fetch_all(&state.pg)
        .await?;

        let mut grouped: HashMap<String, Vec<SlimMediaItem>> = HashMap::new();
        for row in rows {
            grouped
                .entry(row.project_airtable_id.clone())
                .or_default()
                .push(SlimMediaItem {
                    project_id: row.project_id,
                    mime_type: row.mime_type,
                    url: row.url,
                    thumb_large_url: row.thumb_large_url,
                });
        }
        for airtable_id in &missing_airtable_ids {
            let items = grouped.remove(airtable_id).unwrap_or_default();
            cache_slim_items(&mut conn, &cache_key_for_slim(airtable_id), &items).await?;
            if let Some(origin) = airtable_origin.remove(airtable_id) {
                media.insert(origin, items);
            }
        }
    }

    Ok(Json(MediaBatchResponse { media }))
}

fn cache_key_for_full(id: &str) -> String {
    format!("media_items:full:{}", id)
}

fn cache_key_for_slim(id: &str) -> String {
    format!("media_items:slim:{}", id)
}

#[instrument(skip(state))]
async fn get_media_items(state: &AppState, id: &str) -> Result<Vec<MediaItem>, AppError> {
    let cache_key = cache_key_for_full(id);
    let mut conn = state.redis.get().await?;

    if let Ok(Some(cached)) = conn.get::<_, Option<String>>(&cache_key).await {
        if cached == NULL_MEDIA_SENTINEL {
            return Ok(Vec::new());
        }
        if let Ok(items) = serde_json::from_str::<Vec<MediaItem>>(&cached) {
            return Ok(items);
        }
    }

    let items: Vec<MediaItem> = if let Ok(int_id) = id.parse::<i32>() {
        sqlx::query_as!(
            MediaItem,
            "SELECT id, project_id, airtable_id, filename, mime_type, size_bytes, width, height, url, \
             thumb_small_url, thumb_small_width, thumb_small_height, \
             thumb_large_url, thumb_large_width, thumb_large_height, \
             thumb_full_url, thumb_full_width, thumb_full_height \
             FROM media WHERE project_id = $1 ORDER BY id",
            int_id
        )
        .fetch_all(&state.pg)
        .await?
    } else {
        sqlx::query_as!(
            MediaItem,
            "SELECT m.id, m.project_id, m.airtable_id, m.filename, m.mime_type, m.size_bytes, m.width, m.height, m.url, \
             m.thumb_small_url, m.thumb_small_width, m.thumb_small_height, \
             m.thumb_large_url, m.thumb_large_width, m.thumb_large_height, \
             m.thumb_full_url, m.thumb_full_width, m.thumb_full_height \
             FROM media m INNER JOIN projects p ON p.id = m.project_id \
             WHERE p.airtable_id = $1 ORDER BY m.id",
            id
        )
        .fetch_all(&state.pg)
        .await?
    };

    cache_full_items(&mut conn, &cache_key, items.as_slice()).await?;
    Ok(items)
}

async fn cache_full_items(
    conn: &mut deadpool_redis::Connection,
    cache_key: &str,
    items: &[MediaItem],
) -> Result<(), AppError> {
    let payload = if items.is_empty() {
        NULL_MEDIA_SENTINEL.to_string()
    } else {
        serde_json::to_string(items).map_err(anyhow::Error::from)?
    };
    let _: () = conn.set_ex(cache_key, payload, CACHE_TTL_SECS).await?;
    Ok(())
}

async fn cache_slim_items(
    conn: &mut deadpool_redis::Connection,
    cache_key: &str,
    items: &[SlimMediaItem],
) -> Result<(), AppError> {
    let payload = if items.is_empty() {
        NULL_MEDIA_SENTINEL.to_string()
    } else {
        serde_json::to_string(items).map_err(anyhow::Error::from)?
    };
    let _: () = conn.set_ex(cache_key, payload, CACHE_TTL_SECS).await?;
    Ok(())
}
