use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Redirect, Response};
use deadpool_redis::redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::instrument;
use utoipa::{IntoParams, ToSchema};

use crate::error::AppError;
use crate::state::AppState;

const NULL_MEDIA_SENTINEL: &str = "__NULL_MEDIA__";
const DEFAULT_CACHE_TTL_SECS: u64 = 60 * 60;
const MIN_CACHE_TTL_SECS: u64 = 30;
const MAX_CACHE_TTL_SECS: u64 = 60 * 60;
const EXPIRY_SAFETY_MARGIN_SECS: u64 = 60;

fn airtable_url_expiry_secs(url: &str) -> Option<u64> {
    if !url.contains("airtableusercontent.com") {
        return None;
    }

    let after_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let path_and_query = after_scheme.split_once('/').map(|(_, rest)| rest)?;
    let path = path_and_query
        .split_once('?')
        .map(|(p, _)| p)
        .unwrap_or(path_and_query);

    path.split('/').find_map(|seg| {
        let ms = seg.parse::<u64>().ok()?;
        if ms >= 1_000_000_000_000 {
            Some(ms / 1000)
        } else {
            None
        }
    })
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn seconds_until_expiry(url: &str) -> Option<u64> {
    let expiry = airtable_url_expiry_secs(url)?;
    Some(expiry.saturating_sub(now_secs()))
}

fn cache_ttl_for_urls<'a, I: IntoIterator<Item = &'a str>>(urls: I) -> u64 {
    let min_remaining = urls.into_iter().filter_map(seconds_until_expiry).min();

    match min_remaining {
        Some(0) => MIN_CACHE_TTL_SECS,
        Some(remaining) => remaining
            .saturating_sub(EXPIRY_SAFETY_MARGIN_SECS)
            .clamp(MIN_CACHE_TTL_SECS, MAX_CACHE_TTL_SECS),
        None => DEFAULT_CACHE_TTL_SECS,
    }
}

fn full_item_urls(item: &MediaItem) -> impl Iterator<Item = &str> {
    [
        Some(item.url.as_str()),
        item.thumb_small_url.as_deref(),
        item.thumb_large_url.as_deref(),
        item.thumb_full_url.as_deref(),
    ]
    .into_iter()
    .flatten()
}

fn redirect_cache_control(target_url: &str) -> HeaderValue {
    let remaining = match seconds_until_expiry(target_url) {
        Some(r) => r,
        None => DEFAULT_CACHE_TTL_SECS,
    };

    if remaining <= EXPIRY_SAFETY_MARGIN_SECS {
        return HeaderValue::from_static("no-store");
    }

    let max_age = remaining
        .saturating_sub(EXPIRY_SAFETY_MARGIN_SECS)
        .min(MAX_CACHE_TTL_SECS);

    HeaderValue::try_from(format!("public, max-age={max_age}, s-maxage={max_age}"))
        .unwrap_or_else(|_| HeaderValue::from_static("no-store"))
}

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

#[derive(Debug, Deserialize, ToSchema, IntoParams, Default)]
pub struct MediaRedirectQuery {
    #[serde(default)]
    pub size: Option<MediaSize>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum MediaSize {
    Small,
    Large,
    Full,
    Original,
}

#[utoipa::path(
    get,
    path = "/media/{id}/r",
    params(
        ("id" = String, Path, description = "Project ID or Airtable ID"),
        MediaRedirectQuery,
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
    Query(query): Query<MediaRedirectQuery>,
) -> Result<Response, AppError> {
    let items = get_media_items(&state, &id).await?;

    let Some(first) = items.first() else {
        return Err(AppError::not_found(format!(
            "No image found for id: {}",
            id
        )));
    };

    let target = match query.size {
        Some(MediaSize::Small) => first.thumb_small_url.as_deref().unwrap_or(&first.url),
        Some(MediaSize::Large) => first.thumb_large_url.as_deref().unwrap_or(&first.url),
        Some(MediaSize::Full) => first.thumb_full_url.as_deref().unwrap_or(&first.url),
        Some(MediaSize::Original) | None => &first.url,
    };

    let mut response = Redirect::to(target).into_response();
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, redirect_cache_control(target));
    Ok(response)
}

#[instrument(skip(state))]
async fn get_media_items(state: &AppState, id: &str) -> Result<Vec<MediaItem>, AppError> {
    let cache_key = format!("media_items:{}", id);
    let mut conn = state.redis.get().await?;

    if let Ok(Some(cached)) = conn.get::<_, Option<String>>(&cache_key).await {
        if cached == NULL_MEDIA_SENTINEL {
            return Ok(Vec::new());
        }
        if let Ok(items) = serde_json::from_str::<Vec<MediaItem>>(&cached) {
            if !any_urls_expired(items.iter().flat_map(full_item_urls)) {
                return Ok(items);
            }
            let _: Result<(), _> = conn.del::<_, ()>(&cache_key).await;
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
    let (payload, ttl) = if items.is_empty() {
        (NULL_MEDIA_SENTINEL.to_string(), DEFAULT_CACHE_TTL_SECS)
    } else {
        let ttl = cache_ttl_for_urls(items.iter().flat_map(full_item_urls));
        let payload = serde_json::to_string(items).map_err(anyhow::Error::from)?;
        (payload, ttl)
    };
    let _: () = conn.set_ex(cache_key, payload, ttl).await?;
    Ok(())
}

fn any_urls_expired<'a, I>(urls: I) -> bool
where
    I: IntoIterator<Item = &'a str>,
{
    urls.into_iter()
        .filter_map(seconds_until_expiry)
        .any(|remaining| remaining <= EXPIRY_SAFETY_MARGIN_SECS)
}
