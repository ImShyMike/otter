use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;
use tracing::instrument;
use utoipa::ToSchema;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Clone, Copy)]
struct FieldDef {
    column: &'static str,
    kind: FieldKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum FieldKind {
    Text,
    Int,
    Float,
    Timestamp,
    Bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum Field {
    AirtableId,
    Ysws,
    Country,
    Description,
    GithubUsername,
    DisplayName,
    CodeUrl,
    DemoUrl,
    ArchivedDemo,
    ArchivedRepo,
    Hours,
    TrueHours,
    GithubStars,
    ApprovedAt,
    CreatedAt,
    UpdatedAt,
    HasMedia,
    InferredRepo,
    InferredUsername,
}

impl Field {
    #[rustfmt::skip]
    fn def(self) -> FieldDef {
        match self {
            Field::AirtableId => FieldDef { column: "airtable_id", kind: FieldKind::Text },
            Field::Ysws => FieldDef { column: "ysws", kind: FieldKind::Text },
            Field::Country => FieldDef { column: "country", kind: FieldKind::Text },
            Field::Description => FieldDef { column: "description", kind: FieldKind::Text },
            Field::GithubUsername => FieldDef { column: "github_username", kind: FieldKind::Text },
            Field::DisplayName => FieldDef { column: "display_name", kind: FieldKind::Text },
            Field::CodeUrl => FieldDef { column: "code_url", kind: FieldKind::Text },
            Field::DemoUrl => FieldDef { column: "demo_url", kind: FieldKind::Text },
            Field::ArchivedDemo => FieldDef { column: "archived_demo", kind: FieldKind::Text },
            Field::ArchivedRepo => FieldDef { column: "archived_repo", kind: FieldKind::Text },
            Field::Hours => FieldDef { column: "hours", kind: FieldKind::Int },
            Field::TrueHours => FieldDef { column: "true_hours", kind: FieldKind::Float },
            Field::GithubStars => FieldDef { column: "github_stars", kind: FieldKind::Int },
            Field::ApprovedAt => FieldDef { column: "approved_at", kind: FieldKind::Timestamp },
            Field::CreatedAt => FieldDef { column: "created_at", kind: FieldKind::Timestamp },
            Field::UpdatedAt => FieldDef { column: "updated_at", kind: FieldKind::Timestamp },
            Field::HasMedia => FieldDef { column: "media_url", kind: FieldKind::Bool },
            Field::InferredRepo => FieldDef { column: "inferred_repo", kind: FieldKind::Text },
            Field::InferredUsername => FieldDef { column: "inferred_github_username", kind: FieldKind::Text },
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
enum FilterOp {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    NotContains,
    StartsWith,
    EndsWith,
    IsNull,
    IsNotNull,
}

impl FilterOp {
    fn requires_value(&self) -> bool {
        !matches!(self, FilterOp::IsNull | FilterOp::IsNotNull)
    }

    fn allowed_for(&self, kind: FieldKind) -> bool {
        match self {
            FilterOp::Eq | FilterOp::Neq | FilterOp::IsNull | FilterOp::IsNotNull => true,
            FilterOp::Gt | FilterOp::Gte | FilterOp::Lt | FilterOp::Lte => {
                matches!(
                    kind,
                    FieldKind::Int | FieldKind::Timestamp | FieldKind::Float
                )
            }
            FilterOp::Contains
            | FilterOp::NotContains
            | FilterOp::StartsWith
            | FilterOp::EndsWith => kind == FieldKind::Text,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct Filter {
    field: Field,
    op: FilterOp,
    #[serde(default)]
    value: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct QueryRequest {
    #[serde(default)]
    filters: Vec<Filter>,
    #[serde(default)]
    order_by: Option<Field>,
    #[serde(default)]
    sort_direction: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
    #[serde(default)]
    page: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct QueryRow {
    id: i32,
    airtable_id: String,
    ysws: String,
    approved_at: Option<i64>,
    code_url: Option<String>,
    country: Option<String>,
    demo_url: Option<String>,
    description: Option<String>,
    github_username: Option<String>,
    hours: Option<i32>,
    true_hours: Option<f64>,
    has_media: bool,
    github_stars: i32,
    display_name: Option<String>,
    archived_demo: Option<String>,
    archived_repo: Option<String>,
    inferred_repo: Option<String>,
    inferred_github_username: Option<String>,
    _total: i64,
}

#[derive(Serialize, ToSchema)]
pub struct QueryResult {
    id: i32,
    airtable_id: String,
    ysws: String,
    approved_at: Option<i64>,
    code_url: Option<String>,
    country: Option<String>,
    demo_url: Option<String>,
    description: Option<String>,
    github_username: Option<String>,
    hours: Option<i32>,
    true_hours: Option<f64>,
    has_media: bool,
    github_stars: i32,
    display_name: Option<String>,
    archived_demo: Option<String>,
    archived_repo: Option<String>,
    inferred_repo: Option<String>,
    inferred_username: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct QueryResults {
    data: Vec<QueryResult>,
    total: i64,
    page: i64,
    per_page: i64,
}

#[utoipa::path(
    post,
    path = "/query",
    request_body = QueryRequest,
    responses(
        (status = 200, description = "Query results", body = QueryResults),
        (status = 400, description = "Bad request"),
    )
)]
#[instrument(skip(state, body), fields(filters = body.filters.len(), limit = body.limit))]
pub async fn query(
    State(state): State<AppState>,
    Json(body): Json<QueryRequest>,
) -> Result<Json<QueryResults>, AppError> {
    let limit = body.limit.unwrap_or(25).min(100);
    let page = body.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        "SELECT id, airtable_id, ysws, EXTRACT(EPOCH FROM approved_at)::bigint AS approved_at, code_url, country, \
         demo_url, description, github_username, hours, true_hours, \
         (media_url IS NOT NULL) AS has_media, github_stars, display_name, \
         archived_demo, archived_repo, inferred_repo, inferred_github_username, \
         COUNT(*) OVER() AS _total FROM projects WHERE deleted_at IS NULL",
    );

    for filter in &body.filters {
        let def = filter.field.def();

        if !filter.op.allowed_for(def.kind) {
            return Err(AppError::bad_request(format!(
                "Operator {:?} is not valid for field '{:?}'",
                filter.op, filter.field
            )));
        }

        if filter.op.requires_value() && filter.value.is_none() {
            return Err(AppError::bad_request(format!(
                "Operator {:?} requires a value for field '{:?}'",
                filter.op, filter.field
            )));
        }

        if def.kind == FieldKind::Bool {
            match filter.op {
                FilterOp::IsNull => {
                    qb.push(format_args!(" AND {} IS NULL", def.column));
                }
                FilterOp::IsNotNull => {
                    qb.push(format_args!(" AND {} IS NOT NULL", def.column));
                }
                FilterOp::Eq => {
                    let v = parse_bool(&filter.value)?;
                    if v {
                        qb.push(format_args!(" AND {} IS NOT NULL", def.column));
                    } else {
                        qb.push(format_args!(" AND {} IS NULL", def.column));
                    }
                }
                FilterOp::Neq => {
                    let v = parse_bool(&filter.value)?;
                    if v {
                        qb.push(format_args!(" AND {} IS NULL", def.column));
                    } else {
                        qb.push(format_args!(" AND {} IS NOT NULL", def.column));
                    }
                }
                _ => {
                    return Err(AppError::bad_request(format!(
                        "Operator {:?} is not valid for boolean field '{:?}'",
                        filter.op, filter.field
                    )));
                }
            }
            continue;
        }

        match filter.op {
            FilterOp::IsNull => {
                qb.push(format_args!(" AND {} IS NULL", def.column));
            }
            FilterOp::IsNotNull => {
                qb.push(format_args!(" AND {} IS NOT NULL", def.column));
            }
            FilterOp::Eq => match def.kind {
                FieldKind::Text => {
                    let v = parse_text(&filter.value)?;
                    qb.push(format_args!(" AND {} = ", def.column)).push_bind(v);
                }
                FieldKind::Int => {
                    let v = parse_int(&filter.value)?;
                    qb.push(format_args!(" AND {} = ", def.column)).push_bind(v);
                }
                FieldKind::Timestamp => {
                    let v = parse_text(&filter.value)?;
                    qb.push(format_args!(" AND {} = ", def.column))
                        .push_bind(v)
                        .push("::timestamptz");
                }
                FieldKind::Float => {
                    let v = parse_float(&filter.value)?;
                    qb.push(format_args!(" AND {} = ", def.column)).push_bind(v);
                }
                FieldKind::Bool => unreachable!(),
            },
            FilterOp::Neq => match def.kind {
                FieldKind::Text => {
                    let v = parse_text(&filter.value)?;
                    qb.push(format_args!(" AND {} != ", def.column))
                        .push_bind(v);
                }
                FieldKind::Int => {
                    let v = parse_int(&filter.value)?;
                    qb.push(format_args!(" AND {} != ", def.column))
                        .push_bind(v);
                }
                FieldKind::Timestamp => {
                    let v = parse_text(&filter.value)?;
                    qb.push(format_args!(" AND {} != ", def.column))
                        .push_bind(v)
                        .push("::timestamptz");
                }
                FieldKind::Float => {
                    let v = parse_float(&filter.value)?;
                    qb.push(format_args!(" AND {} != ", def.column))
                        .push_bind(v);
                }
                FieldKind::Bool => unreachable!(),
            },
            FilterOp::Gt => match def.kind {
                FieldKind::Int => {
                    let v = parse_int(&filter.value)?;
                    qb.push(format_args!(" AND {} > ", def.column)).push_bind(v);
                }
                FieldKind::Float => {
                    let v = parse_float(&filter.value)?;
                    qb.push(format_args!(" AND {} > ", def.column)).push_bind(v);
                }
                FieldKind::Timestamp => {
                    let v = parse_text(&filter.value)?;
                    qb.push(format_args!(" AND {} > ", def.column))
                        .push_bind(v)
                        .push("::timestamptz");
                }
                _ => unreachable!(),
            },
            FilterOp::Gte => match def.kind {
                FieldKind::Int => {
                    let v = parse_int(&filter.value)?;
                    qb.push(format_args!(" AND {} >= ", def.column))
                        .push_bind(v);
                }
                FieldKind::Float => {
                    let v = parse_float(&filter.value)?;
                    qb.push(format_args!(" AND {} >= ", def.column))
                        .push_bind(v);
                }
                FieldKind::Timestamp => {
                    let v = parse_text(&filter.value)?;
                    qb.push(format_args!(" AND {} >= ", def.column))
                        .push_bind(v)
                        .push("::timestamptz");
                }
                _ => unreachable!(),
            },
            FilterOp::Lt => match def.kind {
                FieldKind::Int => {
                    let v = parse_int(&filter.value)?;
                    qb.push(format_args!(" AND {} < ", def.column)).push_bind(v);
                }
                FieldKind::Float => {
                    let v = parse_float(&filter.value)?;
                    qb.push(format_args!(" AND {} < ", def.column)).push_bind(v);
                }
                FieldKind::Timestamp => {
                    let v = parse_text(&filter.value)?;
                    qb.push(format_args!(" AND {} < ", def.column))
                        .push_bind(v)
                        .push("::timestamptz");
                }
                _ => unreachable!(),
            },
            FilterOp::Lte => match def.kind {
                FieldKind::Int => {
                    let v = parse_int(&filter.value)?;
                    qb.push(format_args!(" AND {} <= ", def.column))
                        .push_bind(v);
                }
                FieldKind::Float => {
                    let v = parse_float(&filter.value)?;
                    qb.push(format_args!(" AND {} <= ", def.column))
                        .push_bind(v);
                }
                FieldKind::Timestamp => {
                    let v = parse_text(&filter.value)?;
                    qb.push(format_args!(" AND {} <= ", def.column))
                        .push_bind(v)
                        .push("::timestamptz");
                }
                _ => unreachable!(),
            },
            FilterOp::Contains => {
                let v = parse_text(&filter.value)?;
                qb.push(format_args!(" AND {} ILIKE ", def.column))
                    .push_bind(format!("%{}%", escape_like(&v)));
            }
            FilterOp::NotContains => {
                let v = parse_text(&filter.value)?;
                qb.push(format_args!(" AND {} NOT ILIKE ", def.column))
                    .push_bind(format!("%{}%", escape_like(&v)));
            }
            FilterOp::StartsWith => {
                let v = parse_text(&filter.value)?;
                qb.push(format_args!(" AND {} ILIKE ", def.column))
                    .push_bind(format!("{}%", escape_like(&v)));
            }
            FilterOp::EndsWith => {
                let v = parse_text(&filter.value)?;
                qb.push(format_args!(" AND {} ILIKE ", def.column))
                    .push_bind(format!("%{}", escape_like(&v)));
            }
        }
    }

    let order_column = match body.order_by {
        Some(field) => field.def().column,
        None => "approved_at",
    };
    let order_dir = match body.sort_direction.as_deref() {
        Some(d) if d.eq_ignore_ascii_case("asc") => "ASC",
        _ => "DESC",
    };

    qb.push(format_args!(
        " ORDER BY {} {} NULLS LAST LIMIT ",
        order_column, order_dir
    ))
    .push_bind(limit)
    .push(" OFFSET ")
    .push_bind(offset);

    let rows = qb.build_query_as::<QueryRow>().fetch_all(&state.pg).await?;

    let total = rows.first().map(|r| r._total).unwrap_or(0);
    let data = rows
        .into_iter()
        .map(|r| QueryResult {
            id: r.id,
            airtable_id: r.airtable_id,
            ysws: r.ysws,
            approved_at: r.approved_at,
            code_url: r.code_url,
            country: r.country,
            demo_url: r.demo_url,
            description: r.description,
            github_username: r.github_username,
            hours: r.hours,
            true_hours: r.true_hours,
            has_media: r.has_media,
            github_stars: r.github_stars,
            display_name: r.display_name,
            archived_demo: r.archived_demo,
            archived_repo: r.archived_repo,
            inferred_repo: r.inferred_repo,
            inferred_username: r.inferred_github_username,
        })
        .collect();

    Ok(Json(QueryResults {
        data,
        total,
        page,
        per_page: limit,
    }))
}

fn parse_text(value: &Option<serde_json::Value>) -> Result<String, AppError> {
    match value {
        Some(serde_json::Value::String(s)) => Ok(s.clone()),
        Some(v) => Ok(v.to_string()),
        None => Err(AppError::bad_request("Missing filter value")),
    }
}

fn parse_int(value: &Option<serde_json::Value>) -> Result<i64, AppError> {
    match value {
        Some(serde_json::Value::Number(n)) => n
            .as_i64()
            .ok_or_else(|| AppError::bad_request("Expected integer value")),
        Some(serde_json::Value::String(s)) => s
            .parse::<i64>()
            .map_err(|_| AppError::bad_request("Expected integer value")),
        _ => Err(AppError::bad_request("Expected integer value")),
    }
}

fn parse_float(value: &Option<serde_json::Value>) -> Result<f64, AppError> {
    match value {
        Some(serde_json::Value::Number(n)) => n
            .as_f64()
            .ok_or_else(|| AppError::bad_request("Expected float value")),
        Some(serde_json::Value::String(s)) => s
            .parse::<f64>()
            .map_err(|_| AppError::bad_request("Expected float value")),
        _ => Err(AppError::bad_request("Expected float value")),
    }
}

fn parse_bool(value: &Option<serde_json::Value>) -> Result<bool, AppError> {
    match value {
        Some(serde_json::Value::Bool(b)) => Ok(*b),
        Some(serde_json::Value::String(s)) => match s.as_str() {
            "true" | "1" => Ok(true),
            "false" | "0" => Ok(false),
            _ => Err(AppError::bad_request("Expected boolean value")),
        },
        _ => Err(AppError::bad_request("Expected boolean value")),
    }
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
