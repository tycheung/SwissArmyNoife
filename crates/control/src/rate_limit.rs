//! Token-bucket rate limit per principal (`sak062-a`).

use std::collections::HashMap;
use std::time::Instant;

use types::ErrorCode;

#[derive(Debug, Clone)]
struct Bucket {
    tokens: f64,
    last: Instant,
}

/// Simple per-principal token bucket (process-local).
#[derive(Debug)]
pub struct RateLimiter {
    burst: f64,
    refill_per_sec: f64,
    buckets: HashMap<String, Bucket>,
    unlimited: bool,
}

impl RateLimiter {
    /// Build from `SAK_RATE_LIMIT_PER_MIN` (0 = unlimited).
    #[must_use]
    pub fn from_env() -> Self {
        let per_min = std::env::var("SAK_RATE_LIMIT_PER_MIN")
            .ok()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        if per_min <= 0.0 {
            return Self::unlimited();
        }
        Self {
            burst: per_min,
            refill_per_sec: per_min / 60.0,
            buckets: HashMap::new(),
            unlimited: false,
        }
    }

    #[must_use]
    pub fn unlimited() -> Self {
        Self {
            burst: 0.0,
            refill_per_sec: 0.0,
            buckets: HashMap::new(),
            unlimited: true,
        }
    }

    /// Fixed limit for tests.
    #[must_use]
    pub fn with_per_min(per_min: f64) -> Self {
        Self {
            burst: per_min,
            refill_per_sec: per_min / 60.0,
            buckets: HashMap::new(),
            unlimited: false,
        }
    }

    /// Consume one token for `principal`.
    ///
    /// # Errors
    /// Returns [`ErrorCode::PolicyDenied`] when the bucket is exhausted.
    pub fn check(&mut self, principal: &str) -> Result<(), ErrorCode> {
        if self.unlimited {
            return Ok(());
        }
        let now = Instant::now();
        let bucket = self.buckets.entry(principal.to_owned()).or_insert(Bucket {
            tokens: self.burst,
            last: now,
        });
        let elapsed = now.duration_since(bucket.last).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.refill_per_sec).min(self.burst);
        bucket.last = now;
        if bucket.tokens < 1.0 {
            return Err(ErrorCode::PolicyDenied);
        }
        bucket.tokens -= 1.0;
        Ok(())
    }

    /// Human-readable deny reason for MCP/HTTP surfaces.
    #[must_use]
    pub fn deny_message() -> String {
        format!("{}: rate_limit exceeded", ErrorCode::PolicyDenied.as_str())
    }

    /// Snapshot remaining tokens for `principal` (refills; does not consume).
    #[must_use]
    pub fn status(&mut self, principal: &str) -> RateLimitStatus {
        if self.unlimited {
            return RateLimitStatus {
                principal: principal.to_owned(),
                unlimited: true,
                remaining: f64::INFINITY,
                burst: 0.0,
            };
        }
        let now = Instant::now();
        let burst = self.burst;
        let refill = self.refill_per_sec;
        let bucket = self.buckets.entry(principal.to_owned()).or_insert(Bucket {
            tokens: burst,
            last: now,
        });
        let elapsed = now.duration_since(bucket.last).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * refill).min(burst);
        bucket.last = now;
        RateLimitStatus {
            principal: principal.to_owned(),
            unlimited: false,
            remaining: bucket.tokens,
            burst,
        }
    }
}

/// Per-principal rate-limit snapshot (`sak528-c`).
#[derive(Clone, Debug, PartialEq)]
pub struct RateLimitStatus {
    pub principal: String,
    pub unlimited: bool,
    pub remaining: f64,
    pub burst: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn burst_exhaust_denies() {
        let mut lim = RateLimiter::with_per_min(2.0);
        lim.check("alice").expect("1");
        lim.check("alice").expect("2");
        assert_eq!(lim.check("alice"), Err(ErrorCode::PolicyDenied));
    }

    #[test]
    fn unlimited_never_denies() {
        let mut lim = RateLimiter::unlimited();
        for _ in 0..100 {
            lim.check("bob").expect("ok");
        }
    }

    #[test]
    fn principals_are_isolated() {
        let mut lim = RateLimiter::with_per_min(1.0);
        lim.check("a").expect("a");
        lim.check("b").expect("b");
        assert_eq!(lim.check("a"), Err(ErrorCode::PolicyDenied));
    }

    #[test]
    fn refill_restores_after_burst_exhausted() {
        let mut lim = RateLimiter::with_per_min(120.0);
        for _ in 0..120 {
            lim.check("alice").expect("burst");
        }
        assert_eq!(lim.check("alice"), Err(ErrorCode::PolicyDenied));
        std::thread::sleep(Duration::from_millis(600));
        lim.check("alice").expect("refilled");
    }

    #[test]
    fn status_reports_remaining_without_consume() {
        let mut lim = RateLimiter::with_per_min(5.0);
        lim.check("alice").expect("1");
        let s = lim.status("alice");
        assert!(!s.unlimited);
        assert!((s.remaining - 4.0).abs() < 0.01);
        assert!((s.burst - 5.0).abs() < 0.01);
        let s2 = lim.status("alice");
        assert!((s2.remaining - 4.0).abs() < 0.01);
    }

    #[test]
    fn deny_message_is_stable() {
        assert_eq!(
            RateLimiter::deny_message(),
            "policy.denied: rate_limit exceeded"
        );
    }
}
