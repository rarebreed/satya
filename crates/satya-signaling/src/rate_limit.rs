//! Sliding-window anti-bot rate limiter and GCP/Cloud Run client IP extractor.

use axum::http::HeaderMap;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Lock-light in-memory sliding-window rate limiter keyed by `(bucket, identity)`.
#[derive(Debug, Default)]
pub struct RateLimiter {
    windows: RwLock<HashMap<(String, String), VecDeque<Instant>>>,
}

impl RateLimiter {
    /// Creates an empty [`RateLimiter`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Checks whether `(bucket, identity)` is within `limit` requests over `window`,
    /// recording the current timestamp if allowed or returning `Err(retry_after_secs)`.
    pub async fn check_and_record(
        &self,
        bucket: &str,
        identity: &str,
        limit: usize,
        window: Duration,
    ) -> Result<(), u64> {
        if limit == 0 {
            return Err(window.as_secs().max(1));
        }
        let now = Instant::now();
        let cutoff = now.checked_sub(window).unwrap_or(now);
        let key = (bucket.to_string(), identity.to_string());

        let mut guard = self.windows.write().await;
        // Prune completely expired buckets periodically when map grows
        if guard.len() > 2048 {
            guard.retain(|_, q| q.back().is_some_and(|ts| *ts > cutoff));
        }

        let queue = guard.entry(key).or_default();
        while let Some(&front) = queue.front() {
            if front <= cutoff {
                queue.pop_front();
            } else {
                break;
            }
        }

        if queue.len() >= limit {
            let oldest = queue.front().copied().unwrap_or(now);
            let elapsed = now.saturating_duration_since(oldest);
            let remaining = window.saturating_sub(elapsed).as_secs().max(1);
            return Err(remaining);
        }

        queue.push_back(now);
        Ok(())
    }
}

/// Extracts the caller IP address for per-IP registration throttling, honoring
/// `X-Forwarded-For` / `X-Real-IP` behind Cloud Run / GCP HTTP Load Balancers.
#[must_use]
pub fn extract_client_ip(headers: &HeaderMap) -> String {
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok())
        && let Some(first) = xff.split(',').next().map(str::trim).filter(|s| !s.is_empty())
    {
        return first.to_string();
    }
    if let Some(real_ip) = headers
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return real_ip.to_string();
    }
    "local".to_string()
}
