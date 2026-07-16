use std::collections::{HashMap, HashSet};

use axum::Json;
use axum::extract::{Query, State};
use serde::{Deserialize, Serialize};
use time::Date;
use tracing::instrument;
use utoipa::{IntoParams, ToSchema};

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, sqlx::FromRow)]
struct FineProjectRow {
    fine_id: i32,
    transaction_id: String,
    amount_cents: i32,
    fine_ysws: Option<String>,
    memo: String,
    fine_date: Date,
    project_id: Option<i32>,
    airtable_id: Option<String>,
    project_ysws: Option<String>,
    approved_at: Option<i64>,
    code_url: Option<String>,
    country: Option<String>,
    country_code: Option<String>,
    demo_url: Option<String>,
    description: Option<String>,
    github_username: Option<String>,
    hours: Option<i32>,
    true_hours: Option<f64>,
    display_name: Option<String>,
    archived_demo: Option<String>,
    archived_repo: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct FinesQuery {
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    page: Option<i64>,
}

#[derive(Serialize, ts_rs::TS, ToSchema)]
pub struct ProjectSummary {
    pub id: i32,
    pub airtable_id: String,
    pub ysws: String,
    pub approved_at: Option<i64>,
    pub code_url: Option<String>,
    pub country: Option<String>,
    pub country_code: Option<String>,
    pub demo_url: Option<String>,
    pub description: Option<String>,
    pub github_username: Option<String>,
    pub hours: Option<i32>,
    pub true_hours: Option<f64>,
    pub display_name: Option<String>,
    pub archived_demo: Option<String>,
    pub archived_repo: Option<String>,
}

#[derive(Serialize, ts_rs::TS, ToSchema)]
pub struct FineItem {
    pub id: i32,
    pub transaction_id: String,
    pub amount_cents: i32,
    pub ysws: Option<String>,
    pub memo: String,
    pub date: String,
    pub projects: Vec<ProjectSummary>,
}

#[derive(Serialize, ts_rs::TS, ToSchema)]
pub struct FinesResults {
    data: Vec<FineItem>,
    total: i64,
    page: i64,
    per_page: i64,
}

#[utoipa_ts::path(
    get,
    path = "/fines",
    params(FinesQuery),
    responses(
        (status = 200, description = "Fines matched to deleted projects", body = FinesResults),
    )
)]
#[instrument(skip(state), fields(limit = params.limit, page = params.page))]
pub async fn fines(
    State(state): State<AppState>,
    Query(params): Query<FinesQuery>,
) -> Result<Json<FinesResults>, AppError> {
    let limit = params.limit.unwrap_or(50).clamp(1, 100);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM fines")
        .fetch_one(&state.pg)
        .await?;

    let rows = sqlx::query_as::<_, FineProjectRow>(
        r#"
        WITH page_fines AS (
          SELECT
            f.id,
            f.transaction_id,
            f.amount_cents,
            f.ysws,
            f.memo,
            f.date
          FROM fines f
          ORDER BY f.id DESC
          LIMIT $1 OFFSET $2
        )
        SELECT
          f.id AS fine_id,
          f.transaction_id,
          f.amount_cents,
          f.ysws AS fine_ysws,
          f.memo,
          f.date AS fine_date,
          p.id AS project_id,
          p.airtable_id,
          p.ysws AS project_ysws,
          p.approved_at,
          p.code_url,
          p.country,
          p.country_code,
          p.demo_url,
          p.description,
          p.github_username,
          p.hours,
          p.true_hours,
          p.display_name,
          p.archived_demo,
          p.archived_repo
        FROM page_fines f
        LEFT JOIN LATERAL (
          SELECT DISTINCT ON (p.id)
            p.id,
            p.airtable_id,
            p.ysws,
            p.approved_at,
            p.code_url,
            p.country,
            p.country_code,
            p.demo_url,
            p.description,
            p.github_username,
            p.hours,
            p.true_hours,
            p.display_name,
            p.archived_demo,
            p.archived_repo
          FROM projects p
          JOIN project_changes jc
            ON jc.project_id = p.id
            AND jc.is_delete = true
            AND jc.changed_at BETWEEN (f.date - INTERVAL '1 day') AND (f.date + INTERVAL '1 day')
          WHERE f.ysws IS NOT NULL
            AND p.ysws IS NOT NULL
            AND p.ysws = f.ysws
          ORDER BY p.id
        ) p ON TRUE
        ORDER BY f.id DESC, p.id ASC
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pg)
    .await?;

    let mut fines: Vec<FineItem> = Vec::new();
    let mut fine_indexes: HashMap<i32, usize> = HashMap::new();
    let mut seen_projects_by_fine: HashMap<i32, HashSet<i32>> = HashMap::new();

    for row in rows {
        let fine_index = if let Some(index) = fine_indexes.get(&row.fine_id) {
            *index
        } else {
            let index = fines.len();
            fines.push(FineItem {
                id: row.fine_id,
                transaction_id: row.transaction_id.clone(),
                amount_cents: row.amount_cents,
                ysws: row.fine_ysws.clone(),
                memo: row.memo.clone(),
                date: row.fine_date.to_string(),
                projects: Vec::new(),
            });
            fine_indexes.insert(row.fine_id, index);
            index
        };

        if let (Some(project_id), Some(airtable_id), Some(project_ysws)) =
            (row.project_id, row.airtable_id, row.project_ysws)
        {
            let project = ProjectSummary {
                id: project_id,
                airtable_id,
                ysws: project_ysws,
                approved_at: row.approved_at,
                code_url: row.code_url,
                country: row.country,
                country_code: row.country_code,
                demo_url: row.demo_url,
                description: row.description,
                github_username: row.github_username,
                hours: row.hours,
                true_hours: row.true_hours,
                display_name: row.display_name,
                archived_demo: row.archived_demo,
                archived_repo: row.archived_repo,
            };

            let seen_projects = seen_projects_by_fine.entry(row.fine_id).or_default();
            if seen_projects.insert(project_id) {
                fines[fine_index].projects.push(project);
            }
        }
    }

    Ok(Json(FinesResults {
        data: fines,
        total,
        page,
        per_page: limit,
    }))
}
