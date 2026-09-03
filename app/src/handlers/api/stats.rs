use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use sqlx::AssertSqlSafe;
use tracing::instrument;
use utoipa::{IntoParams, ToSchema};

use crate::error::AppError;
use crate::state::AppState;

const UNIQUE_SHIPPERS: &str =
    "COUNT(DISTINCT (p.slack_id, p.github_username, p.inferred_username))";
const HOURS_SUM: &str = "ROUND(COALESCE(SUM(COALESCE(p.true_hours, p.hours::double precision, 0)), 0)::numeric, 2)::double precision";
const WP_SUM: &str = "ROUND((COALESCE(SUM(COALESCE(p.true_hours, p.hours::double precision, 0)), 0) / 10.0)::numeric, 2)::double precision";

#[derive(sqlx::FromRow, Serialize, ts_rs::TS, ToSchema)]
pub struct StatsOverview {
    pub total_projects: i64,
    pub total_hours: f64,
    pub total_wp: f64,
    pub unique_shippers: i64,
    pub total_ysws: i64,
    pub total_countries: i64,
}

#[derive(sqlx::FromRow, Serialize, ts_rs::TS, ToSchema)]
pub struct YswsStats {
    pub ysws: String,
    pub total_projects: i64,
    pub total_hours: f64,
    pub total_wp: f64,
    pub unique_shippers: i64,
}

#[derive(sqlx::FromRow, Serialize, ts_rs::TS, ToSchema)]
pub struct CountryStats {
    pub country_code: String,
    pub total_projects: i64,
    pub total_hours: f64,
    pub total_wp: f64,
    pub unique_shippers: i64,
}

#[derive(sqlx::FromRow, Serialize, ts_rs::TS, ToSchema)]
pub struct YswsCountryStats {
    pub ysws: String,
    pub country_code: String,
    pub total_projects: i64,
    pub total_hours: f64,
    pub total_wp: f64,
    pub unique_shippers: i64,
}

#[derive(Debug, Clone, Copy, Deserialize, ts_rs::TS, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TimeGranularity {
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

impl TimeGranularity {
    /// Valid `date_trunc` field name for this granularity.
    fn as_sql(self) -> &'static str {
        match self {
            TimeGranularity::Day => "day",
            TimeGranularity::Week => "week",
            TimeGranularity::Month => "month",
            TimeGranularity::Quarter => "quarter",
            TimeGranularity::Year => "year",
        }
    }
}

#[derive(sqlx::FromRow, Serialize, ts_rs::TS, ToSchema)]
pub struct ProjectPeriodStats {
    /// `YYYY-MM-DD`, the start of the bucket at the requested granularity.
    pub period: String,
    pub ysws: String,
    pub total_projects: i64,
    pub total_hours: f64,
}

#[derive(sqlx::FromRow, Serialize, ts_rs::TS, ToSchema)]
pub struct FineDayStats {
    /// `YYYY-MM-DD`
    pub date: String,
    pub ysws: Option<String>,
    pub amount_cents: i64,
    pub count: i64,
}

#[derive(Serialize, ts_rs::TS, ToSchema)]
pub struct HistogramBucket {
    pub bucket: String,
    pub count: i64,
}

const HOURS_BUCKET_LABELS: [&str; 9] = [
    "0-1", "1-5", "5-10", "10-25", "25-50", "50-100", "100-250", "250-500", "500+",
];
const SUBMISSIONS_BUCKET_LABELS: [&str; 8] =
    ["1", "2", "3-4", "5-9", "10-19", "20-49", "50-99", "100+"];

#[derive(Serialize, ts_rs::TS, ToSchema)]
pub struct StatsResponse {
    pub overview: StatsOverview,
    pub by_ysws: Vec<YswsStats>,
    pub by_country: Vec<CountryStats>,
    pub projects_by_month: Vec<ProjectPeriodStats>,
    pub fines_by_day: Vec<FineDayStats>,
    pub hours_per_shipper_distribution: Vec<HistogramBucket>,
    pub submissions_per_shipper_distribution: Vec<HistogramBucket>,
}

async fn fetch_overview(pg: &sqlx::PgPool) -> Result<StatsOverview, AppError> {
    let overview = sqlx::query_as(AssertSqlSafe(format!(
        r#"
        SELECT
            COUNT(*) AS total_projects,
            {HOURS_SUM} AS total_hours,
            {WP_SUM} AS total_wp,
            {UNIQUE_SHIPPERS} AS unique_shippers,
            COUNT(DISTINCT p.ysws) AS total_ysws,
            COUNT(DISTINCT p.country_code) AS total_countries
        FROM projects p
        WHERE p.deleted_at IS NULL
        "#
    )))
    .fetch_one(pg)
    .await?;
    Ok(overview)
}

async fn fetch_by_ysws(pg: &sqlx::PgPool) -> Result<Vec<YswsStats>, AppError> {
    let by_ysws = sqlx::query_as(AssertSqlSafe(format!(
        r#"
        SELECT
            p.ysws AS ysws,
            COUNT(*) AS total_projects,
            {HOURS_SUM} AS total_hours,
            {WP_SUM} AS total_wp,
            {UNIQUE_SHIPPERS} AS unique_shippers
        FROM projects p
        WHERE p.deleted_at IS NULL AND p.ysws IS NOT NULL
        GROUP BY p.ysws
        ORDER BY total_projects DESC
        "#
    )))
    .fetch_all(pg)
    .await?;
    Ok(by_ysws)
}

async fn fetch_by_country(pg: &sqlx::PgPool) -> Result<Vec<CountryStats>, AppError> {
    let by_country = sqlx::query_as(AssertSqlSafe(format!(
        r#"
        SELECT
            p.country_code AS country_code,
            COUNT(*) AS total_projects,
            {HOURS_SUM} AS total_hours,
            {WP_SUM} AS total_wp,
            {UNIQUE_SHIPPERS} AS unique_shippers
        FROM projects p
        WHERE p.deleted_at IS NULL AND p.country_code IS NOT NULL
        GROUP BY p.country_code
        ORDER BY total_projects DESC
        "#
    )))
    .fetch_all(pg)
    .await?;
    Ok(by_country)
}

async fn fetch_projects_by_period(
    pg: &sqlx::PgPool,
    granularity: TimeGranularity,
) -> Result<Vec<ProjectPeriodStats>, AppError> {
    let rows = sqlx::query_as(
        r#"
        SELECT
            to_char(date_trunc($1, to_timestamp(p.approved_at)), 'YYYY-MM-DD') AS period,
            p.ysws AS ysws,
            COUNT(*) AS total_projects,
            ROUND(COALESCE(SUM(COALESCE(p.true_hours, p.hours::double precision, 0)), 0)::numeric, 2)::double precision AS total_hours
        FROM projects p
        WHERE p.deleted_at IS NULL AND p.approved_at IS NOT NULL
        GROUP BY date_trunc($1, to_timestamp(p.approved_at)), p.ysws
        ORDER BY date_trunc($1, to_timestamp(p.approved_at))
        "#,
    )
    .bind(granularity.as_sql())
    .fetch_all(pg)
    .await?;
    Ok(rows)
}

async fn fetch_fines_by_day(pg: &sqlx::PgPool) -> Result<Vec<FineDayStats>, AppError> {
    let rows = sqlx::query_as(
        r#"
        SELECT
            to_char(date_trunc('day', f.date), 'YYYY-MM-DD') AS date,
            f.ysws AS ysws,
            SUM(f.amount_cents)::bigint AS amount_cents,
            COUNT(*) AS count
        FROM fines f
        GROUP BY date_trunc('day', f.date), f.ysws
        ORDER BY date_trunc('day', f.date)
        "#,
    )
    .fetch_all(pg)
    .await?;
    Ok(rows)
}

async fn fetch_hours_distribution(pg: &sqlx::PgPool) -> Result<Vec<HistogramBucket>, AppError> {
    let buckets: Vec<(i32, i64)> = sqlx::query_as(
        r#"
        WITH shipper_agg AS (
            SELECT
                COALESCE(p.slack_id, p.github_username, p.inferred_username, 'unknown') AS shipper,
                COALESCE(SUM(COALESCE(p.true_hours, p.hours::double precision, 0)), 0) AS total_hours
            FROM projects p
            WHERE p.deleted_at IS NULL
            GROUP BY p.slack_id, p.github_username, p.inferred_username
        )
        SELECT
            width_bucket(total_hours, ARRAY[1, 5, 10, 25, 50, 100, 250, 500]::double precision[])::int AS bucket_idx,
            COUNT(*)::bigint AS count
        FROM shipper_agg
        GROUP BY bucket_idx
        "#,
    )
    .fetch_all(pg)
    .await?;
    Ok(fill_histogram(&HOURS_BUCKET_LABELS, buckets))
}

async fn fetch_submissions_distribution(
    pg: &sqlx::PgPool,
) -> Result<Vec<HistogramBucket>, AppError> {
    let buckets: Vec<(i32, i64)> = sqlx::query_as(
        r#"
        WITH shipper_agg AS (
            SELECT
                COALESCE(p.slack_id, p.github_username, p.inferred_username, 'unknown') AS shipper,
                COUNT(*) AS total_projects
            FROM projects p
            WHERE p.deleted_at IS NULL
            GROUP BY p.slack_id, p.github_username, p.inferred_username
        )
        SELECT
            width_bucket(total_projects::double precision, ARRAY[2, 3, 5, 10, 20, 50, 100]::double precision[])::int AS bucket_idx,
            COUNT(*)::bigint AS count
        FROM shipper_agg
        GROUP BY bucket_idx
        "#,
    )
    .fetch_all(pg)
    .await?;
    Ok(fill_histogram(&SUBMISSIONS_BUCKET_LABELS, buckets))
}

#[utoipa_ts::path(
    get,
    path = "/stats",
    responses(
        (status = 200, description = "Site-wide YSWS statistics", body = StatsResponse),
    )
)]
#[instrument(skip(state))]
pub async fn stats(State(state): State<AppState>) -> Result<Json<StatsResponse>, AppError> {
    let (
        overview,
        by_ysws,
        by_country,
        projects_by_month,
        fines_by_day,
        hours_per_shipper_distribution,
        submissions_per_shipper_distribution,
    ) = tokio::try_join!(
        fetch_overview(&state.pg),
        fetch_by_ysws(&state.pg),
        fetch_by_country(&state.pg),
        fetch_projects_by_period(&state.pg, TimeGranularity::Month),
        fetch_fines_by_day(&state.pg),
        fetch_hours_distribution(&state.pg),
        fetch_submissions_distribution(&state.pg),
    )?;

    Ok(Json(StatsResponse {
        overview,
        by_ysws,
        by_country,
        projects_by_month,
        fines_by_day,
        hours_per_shipper_distribution,
        submissions_per_shipper_distribution,
    }))
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct ProjectsByTimeQuery {
    pub granularity: TimeGranularity,
}

#[utoipa_ts::path(
    get,
    path = "/stats/projects-by-time",
    params(ProjectsByTimeQuery),
    responses(
        (status = 200, description = "Project approval stats at a given time granularity", body = Vec<ProjectPeriodStats>),
    )
)]
#[instrument(skip(state))]
pub async fn stats_projects_by_time(
    State(state): State<AppState>,
    Query(params): Query<ProjectsByTimeQuery>,
) -> Result<Json<Vec<ProjectPeriodStats>>, AppError> {
    Ok(Json(
        fetch_projects_by_period(&state.pg, params.granularity).await?,
    ))
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct CountryBreakdownQuery {
    pub ysws: String,
}

#[utoipa_ts::path(
    get,
    path = "/stats/country-breakdown",
    params(CountryBreakdownQuery),
    responses(
        (status = 200, description = "Per-country stats for a single YSWS program", body = Vec<YswsCountryStats>),
    )
)]
#[instrument(skip(state))]
pub async fn stats_country_breakdown(
    State(state): State<AppState>,
    Query(params): Query<CountryBreakdownQuery>,
) -> Result<Json<Vec<YswsCountryStats>>, AppError> {
    let rows: Vec<YswsCountryStats> = sqlx::query_as(AssertSqlSafe(format!(
        r#"
        SELECT
            p.ysws AS ysws,
            p.country_code AS country_code,
            COUNT(*) AS total_projects,
            {HOURS_SUM} AS total_hours,
            {WP_SUM} AS total_wp,
            {UNIQUE_SHIPPERS} AS unique_shippers
        FROM projects p
        WHERE p.deleted_at IS NULL AND p.ysws = $1 AND p.country_code IS NOT NULL
        GROUP BY p.ysws, p.country_code
        ORDER BY total_projects DESC
        "#
    )))
    .bind(&params.ysws)
    .fetch_all(&state.pg)
    .await?;
    Ok(Json(rows))
}

fn fill_histogram(labels: &[&str], buckets: Vec<(i32, i64)>) -> Vec<HistogramBucket> {
    let mut counts = vec![0i64; labels.len()];
    for (idx, count) in buckets {
        if let Some(slot) = counts.get_mut(idx as usize) {
            *slot = count;
        }
    }
    labels
        .iter()
        .zip(counts)
        .map(|(label, count)| HistogramBucket {
            bucket: label.to_string(),
            count,
        })
        .collect()
}
