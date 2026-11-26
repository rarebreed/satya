//! Sliding-window anti-bot rate limiter, trusted-proxy client IP extractor, and constant-time equality.

use axum::http::HeaderMap;
use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const MAX_RATE_LIMIT_BUCKETS: usize = 16_384;

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
        // Bound map growth against IP/username spray DoS
        if guard.len() >= MAX_RATE_LIMIT_BUCKETS && !guard.contains_key(&key) {
            let excess = guard.len() - MAX_RATE_LIMIT_BUCKETS + 1;
            let mut oldest: Vec<((String, String), Instant)> = guard
                .iter()
                .map(|(k, q)| (k.clone(), q.back().copied().unwrap_or(now)))
                .collect();
            oldest.sort_by_key(|(_, ts)| *ts);
            for (evict_key, _) in oldest.into_iter().take(excess) {
                guard.remove(&evict_key);
            }
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

/// Returns the configured number of trusted reverse-proxy hops (`SATYA_TRUSTED_PROXY_HOPS`, default `0`).
#[must_use]
pub fn trusted_proxy_hops() -> usize {
    std::env::var("SATYA_TRUSTED_PROXY_HOPS")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(0)
}

/// Extracts the caller IP address for per-IP throttling.
///
/// Uses `peer_ip` directly when `SATYA_TRUSTED_PROXY_HOPS` is `0` (default) so spoofed
/// `X-Forwarded-For` / `X-Real-IP` headers cannot bypass rate limits. When `trusted_hops > 0`,
/// selects the rightmost proxy-appended hop from `X-Forwarded-For`.
#[must_use]
pub fn extract_client_ip(headers: &HeaderMap, peer_ip: Option<IpAddr>) -> String {
    let hops = trusted_proxy_hops();
    if hops > 0
        && let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok())
    {
        let entries: Vec<&str> = xff
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if !entries.is_empty() {
            let idx = entries.len().saturating_sub(hops);
            return entries[idx].to_string();
        }
    }
    peer_ip
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "local".to_string())
}

/// Constant-time string comparison for bearer tokens and HMAC hex signatures.
#[must_use]
pub fn ct_eq_str(a: &str, b: &str) -> bool {
    let ab = a.as_bytes();
    let bb = b.as_bytes();
    let len_diff = ab.len() ^ bb.len();
    let max_len = ab.len().max(bb.len());
    let mut acc: usize = len_diff;
    for i in 0..max_len {
        let x = ab.get(i).copied().unwrap_or(0);
        let y = bb.get(i).copied().unwrap_or(0);
        acc |= usize::from(x ^ y);
    }
    std::hint::black_box(acc) == 0
}
