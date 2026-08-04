use std::{collections::HashSet, pin::Pin, time::Duration};

use serde::Deserialize;
use sqlx::PgPool;
use time::{Date, Month};
use tracing::{Instrument, info, instrument};

use crate::utils::http;

const HCB_TRANSACTIONS_URL: &str =
    "https://hcb.hackclub.com/api/v3/organizations/org_NOuVez/transactions?per_page=100&page=";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const PAGE_SIZE: usize = 100;

#[derive(Deserialize)]
struct Transaction {
    id: String,
    amount_cents: i32,
    memo: String,
    date: String,
}

/// Extracts YSWS project name from a fine memo.
/// - "Transfer from (Fines to )YSWS - ProjectName" → "ProjectName"
/// - "Transfer from ProjectName - YSWS" → "ProjectName"
/// - "Transfer from ProjectName" → "ProjectName"
pub fn extract_ysws_from_memo(memo: &str) -> Option<String> {
    memo.strip_prefix("Transfer from")
        .or_else(|| memo.strip_prefix("transfer from"))
        .map(str::trim)
        .and_then(|memo| {
            memo.split(['-', '–'])
                .map(str::trim)
                .find(|part| {
                    !part.is_empty()
                        && !part.eq_ignore_ascii_case("ysws")
                        && !part.eq_ignore_ascii_case("fines to ysws")
                })
                .map(str::to_string)
        })
}

fn parse_date(date: &str) -> anyhow::Result<Date> {
    let [year, month, day]: [&str; 3] = date
        .split('-')
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| anyhow::anyhow!("invalid date format: {date}"))?;

    Ok(Date::from_calendar_date(
        year.parse()?,
        Month::try_from(month.parse::<u8>()?)?,
        day.parse()?,
    )?)
}

pub fn run<'a>(pg: &'a PgPool) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
    Box::pin(
        async move {
            info!("starting");

            let http_client = reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()?;

            let mut page = 1;
            let mut inserted = 0;

            loop {
                let transactions = fetch_transactions(&http_client, page).await?;

                if transactions.is_empty() {
                    info!("no more transactions on page {page}");
                    break;
                }

                let inbound_transactions: Vec<&Transaction> = transactions
                    .iter()
                    .filter(|transaction| transaction.amount_cents != 0)
                    .collect();

                let ids: Vec<&str> = inbound_transactions
                    .iter()
                    .map(|transaction| transaction.id.as_str())
                    .collect();

                let existing_ids: HashSet<String> = sqlx::query_scalar(
                    "SELECT transaction_id FROM fines WHERE transaction_id = ANY($1)",
                )
                .bind(&ids)
                .fetch_all(pg)
                .await?
                .into_iter()
                .collect();

                let new_transactions: Vec<&Transaction> = inbound_transactions
                    .iter()
                    .filter(|transaction| !existing_ids.contains(&transaction.id))
                    .copied()
                    .collect();

                if !new_transactions.is_empty() {
                    inserted += insert_transactions(&new_transactions, pg).await?;
                    info!(
                        page,
                        inserted = new_transactions.len(),
                        "imported fines page"
                    );
                } else if !inbound_transactions.is_empty() {
                    info!(page, "page fully caught up");
                    break;
                }

                if transactions.len() < PAGE_SIZE {
                    break;
                }

                page += 1;
            }

            info!(inserted, "done");

            Ok(())
        }
        .instrument(tracing::info_span!("fines_data")),
    )
}

#[instrument(skip_all)]
async fn fetch_transactions(
    http_client: &reqwest::Client,
    page: i32,
) -> anyhow::Result<Vec<Transaction>> {
    let url = format!("{HCB_TRANSACTIONS_URL}{page}");
    let body = http::fetch_with_retries(3, || http_client.get(&url))
        .await?
        .text()
        .await?;

    Ok(serde_json::from_str(&body)?)
}

#[instrument(skip_all)]
async fn insert_transactions(transactions: &[&Transaction], pg: &PgPool) -> anyhow::Result<u64> {
    let mut tx = pg.begin().await?;

    let mut inserted = 0;

    for transaction in transactions {
        sqlx::query(
            "INSERT INTO fines (transaction_id, amount_cents, ysws, memo, date) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(&transaction.id)
        .bind(transaction.amount_cents)
        .bind(extract_ysws_from_memo(&transaction.memo))
        .bind(&transaction.memo)
        .bind(parse_date(&transaction.date)?)
        .execute(&mut *tx)
        .await?;
        inserted += 1;
    }

    tx.commit().await?;
    Ok(inserted)
}
