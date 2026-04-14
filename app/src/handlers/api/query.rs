use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SearchQuery {
    #[serde(default)]
    min_hours: Option<i64>,
    #[serde(default)]
    max_hours: Option<i64>,
    #[serde(default)]
    ysws: Option<String>,
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    order_by: Option<String>,
    #[serde(default)]
    sort_direction: Option<String>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct SearchResult {
    id: i32,
    airtable_id: String,
    ysws: String,
    approved_at: Option<String>,
    code_url: Option<String>,
    country: Option<String>,
    demo_url: Option<String>,
    description: Option<String>,
    github_username: Option<String>,
    hours: Option<i32>,
    screenshot_url: Option<String>,
    github_stars: i32,
    display_name: Option<String>,
    archived_demo: Option<String>,
    archived_repo: Option<String>,
}

pub async fn query(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<Json<Vec<SearchResult>>, AppError> {
    let limit = params.limit.unwrap_or(10).min(100);
    let has_filter = params.min_hours.is_some()
        || params.max_hours.is_some()
        || params.ysws.is_some()
        || params.country.is_some()
        || params.description.is_some()
        || params.username.is_some()
        || params.order_by.is_some();

    if !has_filter {
        return Err(AppError::bad_request(
            "At least one query parameter is required",
        ));
    }

    let mut query_builder = QueryBuilder::new(
        "SELECT id, airtable_id, ysws, approved_at::text AS approved_at, code_url, country, demo_url, description, github_username, hours, screenshot_url, github_stars, display_name, archived_demo, archived_repo FROM projects WHERE deleted_at IS NULL",
    );

    if let Some(min_hours) = params.min_hours {
        query_builder.push(" AND hours >= ").push_bind(min_hours);
    }

    if let Some(max_hours) = params.max_hours {
        query_builder.push(" AND hours <= ").push_bind(max_hours);
    }

    if let Some(ysws) = params.ysws {
        query_builder.push(" AND ysws = ").push_bind(ysws);
    }

    if let Some(country) = params.country {
        query_builder.push(" AND country = ").push_bind(country);
    }

    if let Some(description) = params.description {
        query_builder
            .push(" AND description ILIKE ")
            .push_bind(format!("%{}%", description));
    }

    if let Some(username) = params.username {
        query_builder
            .push(" AND github_username ILIKE ")
            .push_bind(format!("%{}%", username));
    }

    let order_column = match params.order_by.as_deref() {
        Some("hours") => "hours",
        Some("github_stars") => "github_stars",
        _ => "approved_at",
    };
    let order_direction = match params.sort_direction.as_deref() {
        Some(direction) if direction.eq_ignore_ascii_case("desc") => "DESC",
        _ => "ASC",
    };

    query_builder
        .push(" ORDER BY ")
        .push(order_column)
        .push(" ")
        .push(order_direction)
        .push(" LIMIT ")
        .push_bind(limit);

    let results = query_builder
        .build_query_as::<SearchResult>()
        .fetch_all(&state.pg)
        .await?;

    Ok(Json(results))
}
