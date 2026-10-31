//! Per-IP sliding window for `POST /orders` (not in Rails; see REPORT.md
//! decision 1). Std only: a mutex around a map of request timestamps.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::stores::evict_oldest_entries;

/// Window length: one minute.
pub const ORDER_WINDOW_MS: i64 = 60 * 1000;
/// `POST /orders` requests allowed per IP inside the window.
pub const ORDER_LIMIT: usize = 10;
/// Tracked IPs before the oldest windows are evicted, so a flood of
/// addresses cannot grow the map without bound.
pub const ORDER_MAX_ENTRIES: usize = 50_000;

/// Per-IP request timestamps for `POST /orders`.
#[derive(Default)]
pub struct OrderLimiter {
    inner: Mutex<HashMap<String, Vec<i64>>>,
}

impl OrderLimiter {
    /// Create an empty limiter.
    pub fn new() -> OrderLimiter {
        OrderLimiter::default()
    }

    /// Record one request from `ip` at `now_ms`.
    ///
    /// `Ok(())` when allowed. `Err(secs)` when the window is full, with the
    /// seconds until its oldest request ages out (at least 1). A refused
    /// request is not recorded, so a flood cannot push the retry time out.
    pub fn check(&self, ip: &str, now_ms: i64) -> Result<(), i64> {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let window_start = now_ms - ORDER_WINDOW_MS;
        if !map.contains_key(ip) && map.len() >= ORDER_MAX_ENTRIES {
            map.retain(|_, times| times.last().is_some_and(|&t| t > window_start));
            evict_oldest_entries(&mut map, ORDER_MAX_ENTRIES - 1, |times| {
                times.last().copied().unwrap_or(0)
            });
        }
        let times = map.entry(ip.to_string()).or_default();
        times.retain(|&t| t > window_start);
        if times.len() >= ORDER_LIMIT {
            let oldest = times.first().copied().unwrap_or(now_ms);
            return Err(((oldest + ORDER_WINDOW_MS - now_ms + 999) / 1000).max(1));
        }
        times.push(now_ms);
        Ok(())
    }

    /// Number of tracked IPs.
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Whether no IP is tracked.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IP: &str = "203.0.113.7";

    #[test]
    fn allows_ten_per_minute() {
        let l = OrderLimiter::new();
        assert!((0..ORDER_LIMIT as i64).all(|i| l.check(IP, 1_000 + i).is_ok()));
    }

    #[test]
    fn refuses_the_eleventh_with_retry_after() {
        let l = OrderLimiter::new();
        for i in 0..ORDER_LIMIT as i64 {
            l.check(IP, 1_000 + i).expect("allowed");
        }
        assert_eq!(l.check(IP, 31_000), Err(30));
    }

    #[test]
    fn window_slides() {
        let l = OrderLimiter::new();
        for _ in 0..ORDER_LIMIT {
            l.check(IP, 1_000).expect("allowed");
        }
        assert!(l.check(IP, 1_000 + ORDER_WINDOW_MS).is_ok());
    }

    #[test]
    fn ips_have_separate_windows() {
        let l = OrderLimiter::new();
        for _ in 0..ORDER_LIMIT {
            l.check(IP, 1_000).expect("allowed");
        }
        assert!(l.check("203.0.113.8", 1_000).is_ok());
    }

    #[test]
    fn map_size_is_bounded() {
        let l = OrderLimiter::new();
        for i in 0..ORDER_MAX_ENTRIES + 5 {
            l.check(&format!("ip{i}"), 1_000).expect("allowed");
        }
        assert_eq!(l.len(), ORDER_MAX_ENTRIES);
    }
}
