use std::{env, pin::Pin, time::Duration};

use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Postgres, QueryBuilder};
use std::future::Future;
use tracing::{Instrument, debug, info, instrument, warn};

const SLACK_USERS_URL: &str = "https://slack.com/api/users.list";
const PAGE_LIMIT: usize = 1000;
const UPSERT_BATCH_SIZE: usize = 150;
const PROGRESS_LOG_EVERY_PAGES: usize = 50;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_ATTEMPTS: usize = 6;

#[derive(Debug, Deserialize)]
struct SlackUsersResponse {
    ok: bool,
    members: Vec<SlackUser>,
    #[serde(default)]
    response_metadata: Option<ResponseMetadata>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseMetadata {
    #[serde(default)]
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
struct SlackUserProfile {
    #[serde(default)]
    avatar_hash: Option<String>,
    #[serde(default)]
    real_name: Option<String>,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    real_name_normalized: Option<String>,
    #[serde(default)]
    display_name_normalized: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    first_name: Option<String>,
    #[serde(default)]
    last_name: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    image_72: Option<String>,
    #[serde(default)]
    image_512: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct SlackUser {
    id: String,
    #[serde(default)]
    team_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    tz: Option<String>,
    #[serde(default)]
    real_name: Option<String>,
    #[serde(default)]
    deleted: bool,
    #[serde(default)]
    is_bot: bool,
    #[serde(default)]
    updated: Option<i64>,
    #[serde(default)]
    profile: SlackUserProfile,
}

pub fn run<'a>(pg: &'a PgPool) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
    Box::pin(
        async move {
            info!("starting");

            let slack_token = env::var("SLACK_BOT_TOKEN")?;
            let http_client = reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()?;

            let mut cursor: Option<String> = None;
            let mut page = 1usize;
            let mut total_users = 0usize;
            let mut modified_users = 0usize;

            loop {
                let response =
                    fetch_users_page(&http_client, &slack_token, cursor.as_deref()).await?;

                if response.members.is_empty() {
                    debug!(page, "slack page returned no users");
                } else {
                    let upserted = upsert_users(pg, &response.members).await?;
                    total_users += response.members.len();
                    modified_users += upserted as usize;
                    debug!(
                        page,
                        fetched = response.members.len(),
                        modified = upserted,
                        total_users,
                        "imported slack users page"
                    );

                    if page.is_multiple_of(PROGRESS_LOG_EVERY_PAGES) {
                        info!(
                            page,
                            total_users, modified_users, "imported slack users progress"
                        );
                    }
                }

                let next_cursor = response
                    .response_metadata
                    .and_then(|metadata| metadata.next_cursor)
                    .unwrap_or_default();

                if next_cursor.trim().is_empty() {
                    break;
                }

                cursor = Some(next_cursor);
                page += 1;
                tokio::time::sleep(Duration::from_secs(2)).await;
            }

            info!(total_users, modified_users, "done");

            Ok(())
        }
        .instrument(tracing::info_span!("slack_data")),
    )
}

#[instrument(skip_all)]
async fn fetch_users_page(
    http_client: &reqwest::Client,
    slack_token: &str,
    cursor: Option<&str>,
) -> anyhow::Result<SlackUsersResponse> {
    let mut attempt = 1usize;

    loop {
        let mut params = vec![("limit", PAGE_LIMIT.to_string())];
        if let Some(cursor) = cursor {
            params.push(("cursor", cursor.to_string()));
        }

        let url = reqwest::Url::parse_with_params(SLACK_USERS_URL, &params)?;

        let request = http_client.get(url).bearer_auth(slack_token);

        let response = request.send().await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(5)
                .max(1);

            warn!(attempt, retry_after, "slack users.list rate limited");
            tokio::time::sleep(Duration::from_secs(retry_after)).await;
            attempt += 1;

            if attempt <= MAX_ATTEMPTS {
                continue;
            }

            anyhow::bail!("slack users.list kept rate limiting after {MAX_ATTEMPTS} attempts");
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if status.is_server_error() && attempt < MAX_ATTEMPTS {
                warn!(attempt, %status, "slack users.list failed, retrying");
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt as u32))).await;
                attempt += 1;
                continue;
            }

            anyhow::bail!("slack users.list returned {status}: {body}");
        }

        let payload: SlackUsersResponse = response.json().await?;

        if !payload.ok {
            if payload.error.as_deref() == Some("ratelimited") && attempt < MAX_ATTEMPTS {
                warn!(attempt, "slack users.list returned ratelimited response");
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt as u32))).await;
                attempt += 1;
                continue;
            }

            anyhow::bail!(
                "slack users.list failed: {}",
                payload.error.unwrap_or_else(|| "unknown_error".to_string())
            );
        }

        return Ok(payload);
    }
}

#[instrument(skip_all)]
async fn upsert_users(pg: &PgPool, users: &[SlackUser]) -> anyhow::Result<u64> {
    let mut tx = pg.begin().await?;
    let mut modified = 0u64;

    for chunk in users.chunks(UPSERT_BATCH_SIZE) {
        let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO slack_users (slack_id, team_id, name, email, tz, real_name, display_name, display_name_normalized, deleted, updated_unix, image_72, image_512) ",
        );

        let rows: Vec<_> = chunk
            .iter()
            .filter(|u| !u.is_bot)
            .map(|user| {
                let email = user.profile.email.clone();
                let tz = user.tz.clone();
                let real_name = user
                    .real_name
                    .clone()
                    .or_else(|| user.profile.real_name.clone());
                let display_name = user.profile.display_name.clone();
                let display_name_normalized = user.profile.display_name_normalized.clone();
                let image_72 = user.profile.image_72.clone();
                let image_512 = user.profile.image_512.clone();

                Ok((
                    user,
                    email,
                    tz,
                    real_name,
                    display_name,
                    display_name_normalized,
                    image_72,
                    image_512,
                ))
            })
            .collect::<Result<_, anyhow::Error>>()?;

        qb.push_values(
            rows.iter(),
            |mut b,
             (
                user,
                email,
                tz,
                real_name,
                display_name,
                display_name_normalized,
                image_72,
                image_512,
            )| {
                b.push_bind(&user.id)
                    .push_bind(&user.team_id)
                    .push_bind(&user.name)
                    .push_bind(email.as_deref())
                    .push_bind(tz.as_deref())
                    .push_bind(real_name.as_deref())
                    .push_bind(display_name.as_deref())
                    .push_bind(display_name_normalized.as_deref())
                    .push_bind(user.deleted)
                    .push_bind(user.updated)
                    .push_bind(image_72.as_deref())
                    .push_bind(image_512.as_deref());
            },
        );

        qb.push(
            " ON CONFLICT (slack_id) DO UPDATE SET \
                team_id = EXCLUDED.team_id, \
                name = EXCLUDED.name, \
                email = EXCLUDED.email, \
                tz = EXCLUDED.tz, \
                real_name = EXCLUDED.real_name, \
                display_name = EXCLUDED.display_name, \
                display_name_normalized = EXCLUDED.display_name_normalized, \
                deleted = EXCLUDED.deleted, \
                updated_unix = EXCLUDED.updated_unix, \
                image_72 = EXCLUDED.image_72, \
                image_512 = EXCLUDED.image_512, \
                updated_at = NOW() \
                WHERE slack_users.team_id IS DISTINCT FROM EXCLUDED.team_id \
                OR slack_users.name IS DISTINCT FROM EXCLUDED.name \
                OR slack_users.email IS DISTINCT FROM EXCLUDED.email \
                OR slack_users.tz IS DISTINCT FROM EXCLUDED.tz \
                OR slack_users.real_name IS DISTINCT FROM EXCLUDED.real_name \
                OR slack_users.display_name IS DISTINCT FROM EXCLUDED.display_name \
                OR slack_users.display_name_normalized IS DISTINCT FROM EXCLUDED.display_name_normalized \
                OR slack_users.deleted IS DISTINCT FROM EXCLUDED.deleted \
                OR slack_users.updated_unix IS DISTINCT FROM EXCLUDED.updated_unix \
                OR slack_users.image_72 IS DISTINCT FROM EXCLUDED.image_72 \
                OR slack_users.image_512 IS DISTINCT FROM EXCLUDED.image_512",
        );

        modified += qb
            .build()
            .persistent(false)
            .execute(&mut *tx)
            .await?
            .rows_affected();
    }

    tx.commit().await?;
    Ok(modified)
}
