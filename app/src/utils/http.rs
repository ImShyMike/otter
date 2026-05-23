use std::time::Duration;

use tracing::warn;

/// Sends a request with up to `retries` attempts and exponential backoff
pub async fn fetch_with_retries<F>(
    retries: u32,
    mut request: F,
) -> reqwest::Result<reqwest::Response>
where
    F: FnMut() -> reqwest::RequestBuilder,
{
    let mut last_err = None;
    for attempt in 1..=retries {
        match request().send().await.and_then(|r| r.error_for_status()) {
            Ok(resp) => return Ok(resp),
            Err(e) if attempt < retries => {
                warn!(attempt, "request failed, retrying: {e}");
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt))).await;
                last_err = Some(e);
            }
            Err(e) => return Err(e),
        }
    }
    Err(last_err.unwrap())
}
