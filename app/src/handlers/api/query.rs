use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use sqlx::QueryBuilder;

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

fn lookup_field(name: &str) -> Option<FieldDef> {
    match name {
        "airtable_id" => Some(FieldDef {
            column: "airtable_id",
            kind: FieldKind::Text,
        }),
        "ysws" => Some(FieldDef {
            column: "ysws",
            kind: FieldKind::Text,
        }),
        "country" => Some(FieldDef {
            column: "country",
            kind: FieldKind::Text,
        }),
        "description" => Some(FieldDef {
            column: "description",
            kind: FieldKind::Text,
        }),
        "github_username" => Some(FieldDef {
            column: "github_username",
            kind: FieldKind::Text,
        }),
        "display_name" => Some(FieldDef {
            column: "display_name",
            kind: FieldKind::Text,
        }),
        "code_url" => Some(FieldDef {
            column: "code_url",
            kind: FieldKind::Text,
        }),
        "demo_url" => Some(FieldDef {
            column: "demo_url",
            kind: FieldKind::Text,
        }),
        "archived_demo" => Some(FieldDef {
            column: "archived_demo",
            kind: FieldKind::Text,
        }),
        "archived_repo" => Some(FieldDef {
            column: "archived_repo",
            kind: FieldKind::Text,
        }),
        "hours" => Some(FieldDef {
            column: "hours",
            kind: FieldKind::Int,
        }),
        "true_hours" => Some(FieldDef {
            column: "true_hours",
            kind: FieldKind::Float,
        }),
        "github_stars" => Some(FieldDef {
            column: "github_stars",
            kind: FieldKind::Int,
        }),
        "approved_at" => Some(FieldDef {
            column: "approved_at",
            kind: FieldKind::Timestamp,
        }),
        "created_at" => Some(FieldDef {
            column: "created_at",
            kind: FieldKind::Timestamp,
        }),
        "updated_at" => Some(FieldDef {
            column: "updated_at",
            kind: FieldKind::Timestamp,
        }),
        "has_screenshot" => Some(FieldDef {
            column: "screenshot_url",
            kind: FieldKind::Bool,
        }),
        _ => None,
    }
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
pub struct Filter {
    field: String,
    op: FilterOp,
    #[serde(default)]
    value: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    #[serde(default)]
    filters: Vec<Filter>,
    #[serde(default)]
    order_by: Option<String>,
    #[serde(default)]
    sort_direction: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Serialize, sqlx::FromRow)]
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
    has_screenshot: bool,
    github_stars: i32,
    display_name: Option<String>,
    archived_demo: Option<String>,
    archived_repo: Option<String>,
}

pub async fn query(
    State(state): State<AppState>,
    Json(body): Json<QueryRequest>,
) -> Result<Json<Vec<QueryResult>>, AppError> {
    let limit = body.limit.unwrap_or(10).min(100);

    let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        "SELECT id, airtable_id, ysws, EXTRACT(EPOCH FROM approved_at)::bigint AS approved_at, code_url, country, \
         demo_url, description, github_username, hours, true_hours, \
         (screenshot_url IS NOT NULL) AS has_screenshot, github_stars, display_name, \
         archived_demo, archived_repo FROM projects WHERE deleted_at IS NULL",
    );

    for filter in &body.filters {
        let def = lookup_field(&filter.field)
            .ok_or_else(|| AppError::bad_request(format!("Unknown field: {}", filter.field)))?;

        if !filter.op.allowed_for(def.kind) {
            return Err(AppError::bad_request(format!(
                "Operator {:?} is not valid for field '{}'",
                filter.op, filter.field
            )));
        }

        if filter.op.requires_value() && filter.value.is_none() {
            return Err(AppError::bad_request(format!(
                "Operator {:?} requires a value for field '{}'",
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
                        "Operator {:?} is not valid for boolean field '{}'",
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

    let order_column = match body.order_by.as_deref() {
        Some(field) => {
            let def = lookup_field(field)
                .ok_or_else(|| AppError::bad_request(format!("Unknown order_by field: {field}")))?;
            def.column
        }
        None => "approved_at",
    };
    let order_dir = match body.sort_direction.as_deref() {
        Some(d) if d.eq_ignore_ascii_case("desc") => "DESC",
        _ => "ASC",
    };

    qb.push(format_args!(
        " ORDER BY {} {} LIMIT ",
        order_column, order_dir
    ))
    .push_bind(limit);

    let results = qb
        .build_query_as::<QueryResult>()
        .fetch_all(&state.pg)
        .await?;

    Ok(Json(results))
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
