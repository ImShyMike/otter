use std::sync::LazyLock;

use axum::Json;
use axum::extract::{Path, Query, State};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::AssertSqlSafe;
use tracing::instrument;
use utoipa::{IntoParams, ToSchema};

use crate::error::AppError;
use crate::handlers::api::ProjectItem;
use crate::state::AppState;

/// Slack IDs look like `U012ABCDEF` / `W012ABCDEF`.
static SLACK_ID_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[UW][A-Z0-9]{6,20}$").unwrap());

/// Attach Slack profile data to a username only when one account clearly dominates.
const PRIMARY_MIN_SHARE: f64 = 0.5;
const PRIMARY_MAX_ACCOUNTS: i64 = 3;

const PROJECT_COLUMNS: &str = "p.id, p.airtable_id, p.ysws, p.approved_at, p.code_url, p.country, \
     p.country_code, p.demo_url, p.description, p.slack_id, p.github_username, p.hours, \
     p.true_hours, p.has_media, p.github_stars, p.display_name, p.archived_demo, p.archived_repo, \
     p.inferred_repo, p.inferred_username, p.is_github_url, p.preview_blurhash";

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct UserQuery {
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    page: Option<i64>,
}

/// How the identifier in the URL was resolved to a set of projects.
#[derive(Debug, Clone, Copy, Serialize, ts_rs::TS, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UserMatch {
    SlackId,
    Username,
}

#[derive(Serialize, ts_rs::TS, ToSchema)]
pub struct SlackAccount {
    pub slack_id: String,
    pub handle: Option<String>,
    pub display_name: Option<String>,
    pub real_name: Option<String>,
    pub image: Option<String>,
    pub project_count: i64,
}

#[derive(Serialize, ts_rs::TS, ToSchema)]
pub struct UserProfile {
    /// The identifier that was looked up, normalized (Slack IDs are uppercased)
    pub identifier: String,
    pub matched_by: UserMatch,
    /// The Slack account this page represents, when one can be picked confidently
    pub slack: Option<SlackAccount>,
    /// True when the identifier maps to more than one Slack account
    pub ambiguous: bool,
    /// Total number of distinct Slack accounts behind the matched projects
    pub total_slack_accounts: i64,
    /// Every Slack account behind the matched projects, most projects first
    pub slack_accounts: Vec<SlackAccount>,
    /// Every GitHub + inferred username across the matched projects, most common first
    pub usernames: Vec<String>,
    /// Subset of `usernames` known to come from a GitHub URL
    pub github_usernames: Vec<String>,
    /// Every Airtable display name across the matched projects, most common first
    pub display_names: Vec<String>,
    pub ysws: Vec<String>,
    pub total_projects: i64,
    pub total_hours: f64,
    pub total_stars: i64,
    pub first_approved_at: Option<i64>,
    pub last_approved_at: Option<i64>,
    pub projects: Vec<ProjectItem>,
    pub page: i64,
    pub per_page: i64,
}

#[derive(sqlx::FromRow)]
struct UserAggregate {
    total_projects: i64,
    total_hours: f64,
    total_stars: i64,
    first_approved_at: Option<i64>,
    last_approved_at: Option<i64>,
    slack_tagged_projects: i64,
    total_slack_accounts: i64,
    usernames: Vec<String>,
    github_usernames: Vec<String>,
    display_names: Vec<String>,
    ysws: Vec<String>,
}

#[derive(sqlx::FromRow)]
struct SlackAccountRow {
    slack_id: String,
    name: Option<String>,
    display_name: Option<String>,
    real_name: Option<String>,
    image_512: Option<String>,
    image_72: Option<String>,
    project_count: i64,
}

impl From<&SlackAccountRow> for SlackAccount {
    fn from(row: &SlackAccountRow) -> Self {
        Self {
            slack_id: row.slack_id.clone(),
            handle: blank_to_none(row.name.as_deref()),
            display_name: blank_to_none(row.display_name.as_deref()),
            real_name: blank_to_none(row.real_name.as_deref()),
            image: blank_to_none(row.image_512.as_deref())
                .or_else(|| blank_to_none(row.image_72.as_deref())),
            project_count: row.project_count,
        }
    }
}

/// Escape text for ILIKE statements
fn escape_like(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn blank_to_none(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
}

fn predicate_for(matched_by: UserMatch) -> &'static str {
    match matched_by {
        UserMatch::SlackId => "p.slack_id = $1",
        UserMatch::Username => {
            "(p.github_username ILIKE $1 ESCAPE '\\' OR p.inferred_username ILIKE $1 ESCAPE '\\')"
        }
    }
}

fn matched_projects(predicate: &str) -> String {
    format!("SELECT p.* FROM projects p WHERE p.deleted_at IS NULL AND {predicate}")
}

async fn fetch_aggregate(
    state: &AppState,
    predicate: &str,
    bound_value: &str,
) -> Result<UserAggregate, AppError> {
    let matched = matched_projects(predicate);

    let aggregate = sqlx::query_as(AssertSqlSafe(format!(
        r#"
        WITH matched AS ({matched}),
        name_counts AS (
            SELECT github_username AS name, COUNT(*) AS n, bool_or(is_github_url) AS from_github
            FROM matched WHERE github_username IS NOT NULL GROUP BY 1
            UNION ALL
            SELECT inferred_username, COUNT(*), bool_or(is_github_url)
            FROM matched WHERE inferred_username IS NOT NULL GROUP BY 1
        ),
        names AS (
            SELECT name, SUM(n) AS n, bool_or(from_github) AS from_github
            FROM name_counts GROUP BY 1
        ),
        display_name_counts AS (
            SELECT display_name AS name, COUNT(*) AS n
            FROM matched WHERE display_name IS NOT NULL GROUP BY 1
        ),
        ysws_counts AS (
            SELECT ysws AS name, COUNT(*) AS n FROM matched GROUP BY 1
        )
        SELECT
            (SELECT COUNT(*) FROM matched) AS total_projects,
            (SELECT COALESCE(SUM(COALESCE(true_hours, hours::double precision)), 0)
                FROM matched)::double precision AS total_hours,
            (SELECT COALESCE(SUM(github_stars), 0) FROM matched)::bigint AS total_stars,
            (SELECT MIN(approved_at) FROM matched) AS first_approved_at,
            (SELECT MAX(approved_at) FROM matched) AS last_approved_at,
            (SELECT COUNT(*) FROM matched WHERE slack_id IS NOT NULL) AS slack_tagged_projects,
            (SELECT COUNT(DISTINCT slack_id) FROM matched) AS total_slack_accounts,
            COALESCE((SELECT ARRAY_AGG(name ORDER BY n DESC, name) FROM names), '{{}}'::text[])
                AS usernames,
            COALESCE(
                (SELECT ARRAY_AGG(name ORDER BY n DESC, name) FROM names WHERE from_github),
                '{{}}'::text[]
            ) AS github_usernames,
            COALESCE(
                (SELECT ARRAY_AGG(name ORDER BY n DESC, name) FROM display_name_counts),
                '{{}}'::text[]
            ) AS display_names,
            COALESCE((SELECT ARRAY_AGG(name ORDER BY n DESC, name) FROM ysws_counts), '{{}}'::text[])
                AS ysws
        "#
    )))
    .bind(bound_value)
    .fetch_one(&state.pg)
    .await?;

    Ok(aggregate)
}

/// The account whose Slack handle is the looked up username (fallback to the dominant one)
fn pick_primary_account<'a>(
    accounts: &'a [SlackAccountRow],
    identifier: &str,
    slack_tagged_projects: i64,
    total_slack_accounts: i64,
) -> Option<&'a SlackAccountRow> {
    let named = accounts.iter().find(|account| {
        [account.name.as_deref(), account.display_name.as_deref()]
            .into_iter()
            .flatten()
            .any(|name| name.eq_ignore_ascii_case(identifier))
    });
    if named.is_some() {
        return named;
    }

    let top = accounts.first()?;
    let share = if slack_tagged_projects > 0 {
        top.project_count as f64 / slack_tagged_projects as f64
    } else {
        0.0
    };

    (total_slack_accounts <= PRIMARY_MAX_ACCOUNTS && share >= PRIMARY_MIN_SHARE).then_some(top)
}

#[utoipa_ts::path(
    get,
    path = "/user/{id}",
    params(
        ("id" = String, Path, description = "Slack ID, inferred username or GitHub username"),
        UserQuery,
    ),
    responses(
        (status = 200, description = "User profile and their projects", body = UserProfile),
        (status = 404, description = "Not found"),
    )
)]
#[instrument(skip(state))]
pub async fn user_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(params): Query<UserQuery>,
) -> Result<Json<UserProfile>, AppError> {
    let id = id.trim();
    if id.is_empty() {
        return Err(AppError::bad_request("empty user identifier"));
    }

    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    // always try slack id first
    let uppercased = id.to_uppercase();
    let mut lookups = Vec::with_capacity(2);
    if SLACK_ID_RE.is_match(&uppercased) {
        lookups.push((UserMatch::SlackId, uppercased.clone(), uppercased.clone()));
    }
    lookups.push((UserMatch::Username, id.to_string(), escape_like(id)));

    let mut resolved = None;
    for (matched_by, identifier, bound_value) in lookups {
        let predicate = predicate_for(matched_by);
        let aggregate = fetch_aggregate(&state, predicate, &bound_value).await?;
        if aggregate.total_projects > 0 {
            resolved = Some((matched_by, identifier, bound_value, predicate, aggregate));
            break;
        }
    }

    let Some((matched_by, identifier, bound_value, predicate, aggregate)) = resolved else {
        return Err(AppError::not_found("user not found"));
    };

    let matched = matched_projects(predicate);

    let accounts: Vec<SlackAccountRow> = sqlx::query_as(AssertSqlSafe(format!(
        r#"
        WITH matched AS ({matched}),
        counts AS (
            SELECT slack_id, COUNT(*) AS project_count
            FROM matched WHERE slack_id IS NOT NULL
            GROUP BY 1
        )
        SELECT
            c.slack_id,
            su.name,
            su.display_name,
            su.real_name,
            su.image_512,
            su.image_72,
            c.project_count
        FROM counts c
        LEFT JOIN slack_users su ON su.slack_id = c.slack_id
        ORDER BY c.project_count DESC, c.slack_id
        "#
    )))
    .bind(&bound_value)
    .fetch_all(&state.pg)
    .await?;

    let projects: Vec<ProjectItem> = sqlx::query_as(AssertSqlSafe(format!(
        r#"
        SELECT {PROJECT_COLUMNS}
        FROM projects p
        WHERE p.deleted_at IS NULL AND {predicate}
        ORDER BY p.approved_at DESC NULLS LAST, p.id DESC
        LIMIT $2
        OFFSET $3
        "#
    )))
    .bind(&bound_value)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pg)
    .await?;

    let primary = pick_primary_account(
        &accounts,
        &identifier,
        aggregate.slack_tagged_projects,
        aggregate.total_slack_accounts,
    );

    Ok(Json(UserProfile {
        identifier,
        matched_by,
        slack: primary.map(SlackAccount::from),
        ambiguous: aggregate.total_slack_accounts > 1,
        total_slack_accounts: aggregate.total_slack_accounts,
        slack_accounts: accounts.iter().map(SlackAccount::from).collect(),
        usernames: aggregate.usernames,
        github_usernames: aggregate.github_usernames,
        display_names: aggregate.display_names,
        ysws: aggregate.ysws,
        total_projects: aggregate.total_projects,
        total_hours: aggregate.total_hours,
        total_stars: aggregate.total_stars,
        first_approved_at: aggregate.first_approved_at,
        last_approved_at: aggregate.last_approved_at,
        projects,
        page,
        per_page: limit,
    }))
}
