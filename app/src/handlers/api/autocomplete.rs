use axum::Json;
use axum::extract::{Query, State};
use deadpool_redis::Pool;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use utoipa::{IntoParams, ToSchema};

use crate::error::AppError;
use crate::handlers::api::escape_like;
use crate::handlers::api::user::{SlackAccount, SlackAccountRow};
use crate::state::AppState;

const CACHE_TTL_SECONDS: usize = 60 * 5;
/// minimum chars for trigram search
const MIN_CONTAINS_LEN: usize = 3;
const MAX_QUERY_LEN: usize = 64;

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct AutocompleteQuery {
    /// The text typed so far
    q: String,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Serialize, Deserialize, ts_rs::TS, ToSchema, sqlx::FromRow)]
pub struct UsernameSuggestion {
    pub username: String,
    /// Projects this username is attached to
    pub project_count: i64,
    /// True when the username is known to come from a GitHub URL
    pub is_github: bool,
}

/// The `LIKE`/`ILIKE` patterns a query is expanded into
struct Patterns {
    /// The trimmed, lowercased query, for exact match ranking
    normalized: String,
    /// `<query>%`, matched against `lower(column)`
    prefix: String,
    /// `%<query>%`, matched with `ILIKE`, or `None` when the query is too short to index
    contains: Option<String>,
}

fn patterns(query: &str) -> Option<Patterns> {
    let trimmed = query.trim();
    let length = trimmed.chars().count();
    if length == 0 || length > MAX_QUERY_LEN {
        return None;
    }

    let normalized = trimmed.to_lowercase();
    let escaped = escape_like(&normalized);

    Some(Patterns {
        prefix: format!("{escaped}%"),
        contains: (length >= MIN_CONTAINS_LEN).then(|| format!("%{escaped}%")),
        normalized,
    })
}

fn limit_for(limit: Option<i64>) -> i64 {
    limit.unwrap_or(7).clamp(1, 25)
}

fn cache_key(kind: &str, patterns: &Patterns, limit: i64) -> String {
    format!("autocomplete:{kind}:{limit}:{}", patterns.normalized)
}

async fn get_from_cache<T: DeserializeOwned>(redis: &Pool, key: &str) -> Option<T> {
    let mut conn = redis.get().await.ok()?;
    let cached: Option<String> = redis::cmd("GET")
        .arg(key)
        .query_async(&mut *conn)
        .await
        .ok()?;

    let suggestions = serde_json::from_str(&cached?).ok()?;
    debug!(key, "cache hit for autocomplete");
    Some(suggestions)
}

async fn store_in_cache<T: Serialize>(redis: &Pool, key: &str, value: &T) {
    let Ok(json) = serde_json::to_string(value) else {
        return;
    };
    let Ok(mut conn) = redis.get().await else {
        return;
    };
    let _ = redis::cmd("SETEX")
        .arg(key)
        .arg(CACHE_TTL_SECONDS)
        .arg(&json)
        .query_async::<()>(&mut *conn)
        .await;
}

#[utoipa_ts::path(
    get,
    path = "/autocomplete/user",
    params(AutocompleteQuery),
    responses(
        (status = 200, description = "Username suggestions, best match first", body = Vec<UsernameSuggestion>),
    )
)]
#[instrument(skip(state), fields(q = %params.q))]
pub async fn autocomplete_user(
    State(state): State<AppState>,
    Query(params): Query<AutocompleteQuery>,
) -> Result<Json<Vec<UsernameSuggestion>>, AppError> {
    let limit = limit_for(params.limit);
    let Some(patterns) = patterns(&params.q) else {
        return Ok(Json(Vec::new()));
    };

    let key = cache_key("user", &patterns, limit);
    if let Some(cached) = get_from_cache(&state.redis, &key).await {
        return Ok(Json(cached));
    }

    // usernames containing whitespace cannot be typed into the `user:` filter, so they are skipped
    let suggestions: Vec<UsernameSuggestion> = sqlx::query_as(
        r#"
        WITH names AS (
            SELECT p.id, p.github_username AS username, p.is_github_url
            FROM projects p
            WHERE p.deleted_at IS NULL
              AND p.github_username IS NOT NULL
              AND p.github_username !~ '\s'
              AND (
                    lower(p.github_username) LIKE $1 ESCAPE '\'
                    OR ($2::text IS NOT NULL AND p.github_username ILIKE $2 ESCAPE '\')
              )
            UNION ALL
            SELECT p.id, p.inferred_username, p.is_github_url
            FROM projects p
            WHERE p.deleted_at IS NULL
              AND p.inferred_username IS NOT NULL
              AND p.inferred_username !~ '\s'
              AND (
                    lower(p.inferred_username) LIKE $1 ESCAPE '\'
                    OR ($2::text IS NOT NULL AND p.inferred_username ILIKE $2 ESCAPE '\')
              )
        ),
        grouped AS (
            SELECT
                lower(username) AS key,
                mode() WITHIN GROUP (ORDER BY username) AS username,
                COUNT(DISTINCT id)::bigint AS project_count,
                COALESCE(bool_or(is_github_url), FALSE) AS is_github
            FROM names
            GROUP BY lower(username)
        )
        SELECT username, project_count, is_github
        FROM grouped
        ORDER BY
            (CASE
                WHEN key = $3 THEN 2
                WHEN key LIKE $1 ESCAPE '\' THEN 1
                ELSE 0
            END) DESC,
            project_count DESC,
            username
        LIMIT $4
        "#,
    )
    .bind(&patterns.prefix)
    .bind(&patterns.contains)
    .bind(&patterns.normalized)
    .bind(limit)
    .fetch_all(&state.pg)
    .await?;

    store_in_cache(&state.redis, &key, &suggestions).await;

    Ok(Json(suggestions))
}

#[utoipa_ts::path(
    get,
    path = "/autocomplete/slack",
    params(AutocompleteQuery),
    responses(
        (status = 200, description = "Slack account suggestions, best match first", body = Vec<SlackAccount>),
    )
)]
#[instrument(skip(state), fields(q = %params.q))]
pub async fn autocomplete_slack(
    State(state): State<AppState>,
    Query(params): Query<AutocompleteQuery>,
) -> Result<Json<Vec<SlackAccount>>, AppError> {
    let limit = limit_for(params.limit);
    let Some(patterns) = patterns(&params.q) else {
        return Ok(Json(Vec::new()));
    };

    let key = cache_key("slack", &patterns, limit);
    if let Some(cached) = get_from_cache(&state.redis, &key).await {
        return Ok(Json(cached));
    }

    // only accounts that own at least one project are suggested
    let rows: Vec<SlackAccountRow> = sqlx::query_as(
        r#"
        WITH counts AS (
            SELECT p.slack_id, COUNT(*)::bigint AS project_count
            FROM projects p
            WHERE p.deleted_at IS NULL AND p.slack_id IS NOT NULL
            GROUP BY 1
        ),
        matched AS (
            SELECT
                su.slack_id,
                su.name,
                su.display_name,
                su.real_name,
                su.image_512,
                su.image_72,
                c.project_count,
                GREATEST(
                    CASE
                        WHEN lower(su.name) = $3
                            OR lower(su.display_name) = $3
                            OR lower(su.real_name) = $3
                            OR lower(su.slack_id) = $3
                        THEN 3
                        ELSE 0
                    END,
                    CASE
                        WHEN lower(su.name) LIKE $1 ESCAPE '\'
                            OR lower(su.display_name) LIKE $1 ESCAPE '\'
                            OR lower(su.real_name) LIKE $1 ESCAPE '\'
                            OR lower(su.slack_id) LIKE $1 ESCAPE '\'
                        THEN 2
                        ELSE 0
                    END,
                    CASE
                        WHEN $2::text IS NOT NULL AND (
                            su.name ILIKE $2 ESCAPE '\'
                            OR su.display_name ILIKE $2 ESCAPE '\'
                            OR su.real_name ILIKE $2 ESCAPE '\'
                        )
                        THEN 1
                        ELSE 0
                    END
                ) AS match_rank
            FROM counts c
            INNER JOIN slack_users su ON su.slack_id = c.slack_id
            WHERE su.deleted = FALSE
        )
        SELECT slack_id, name, display_name, real_name, image_512, image_72, project_count
        FROM matched
        WHERE match_rank > 0
        ORDER BY match_rank DESC, project_count DESC, slack_id
        LIMIT $4
        "#,
    )
    .bind(&patterns.prefix)
    .bind(&patterns.contains)
    .bind(&patterns.normalized)
    .bind(limit)
    .fetch_all(&state.pg)
    .await?;

    let suggestions: Vec<SlackAccount> = rows.iter().map(SlackAccount::from).collect();
    store_in_cache(&state.redis, &key, &suggestions).await;

    Ok(Json(suggestions))
}
