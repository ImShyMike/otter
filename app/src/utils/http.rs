use std::time::Duration;

use tracing::warn;

/// Fetches the given URL with up to `retries` attempts and exponential backoff
pub async fn fetch_with_retries(
    client: &reqwest::Client,
    url: &str,
    retries: u32,
) -> reqwest::Result<reqwest::Response> {
    let mut last_err = None;
    for attempt in 1..=retries {
        match client.get(url).send().await?.error_for_status() {
            Ok(resp) => return Ok(resp),
            Err(e) if attempt < retries => {
                warn!(attempt, "fetch failed, retrying: {e}");
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
                last_err = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap())
}
