//! A fixed-window request counter, in memory.
//!
//! In memory because the limit is a brake on abuse, not an accounting record:
//! a restart forgiving everybody's count is the correct trade for not putting a
//! write on the hot path of every request. A deployment behind more than one
//! process would need a shared store, and this app is one process.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Requests allowed per key per window.
pub const MAX_REQUESTS: u32 = 1000;

/// The window they are counted over.
pub const WINDOW: Duration = Duration::from_secs(60 * 60);

/// A public share link renders a PDF for whoever holds it, without an account
/// to key on — so it gets its own, much tighter windows on top of the general
/// limit above, one per address and one per CV.
///
/// A cache hit is cheap, but the first visit after every edit is a full Typst
/// compile, so both stay well under what a render worker can actually absorb
/// even if every request happens to miss.
pub const PUBLIC_IP_MAX_REQUESTS: u32 = 30;
/// Higher than the per-IP cap: a link shared somewhere genuinely popular is
/// visited by many different addresses, and that traffic is legitimate in a
/// way that one address alone hammering a link is not.
pub const PUBLIC_CV_MAX_REQUESTS: u32 = 120;

/// Sweep expired keys once the table passes this size. A sweep is O(n) and
/// this bounds how often it runs; below it, the table is small enough that the
/// memory does not matter.
const SWEEP_AT: usize = 4096;

#[derive(Debug, Clone, Copy)]
struct Counter {
    started: Instant,
    hits: u32,
}

#[derive(Clone)]
pub struct RateLimiter {
    counters: Arc<Mutex<HashMap<String, Counter>>>,
    /// Carried rather than read from the constant so a test can build a
    /// limiter it can actually reach the end of — a thousand round trips to
    /// prove the middleware is wired up is a thousand round trips of nothing.
    limit: u32,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::with_limit(MAX_REQUESTS)
    }
}

/// What a caller has left, once a request has been counted against them.
#[derive(Debug)]
pub struct Allowance {
    pub remaining: u32,
    /// Seconds until the window rolls, for `Retry-After`.
    pub reset_in: u64,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_limit(limit: u32) -> Self {
        Self {
            counters: Arc::new(Mutex::new(HashMap::new())),
            limit,
        }
    }

    /// Count one request from `key`.
    ///
    /// `Ok` carries what is left of the allowance; `Err` carries how long the
    /// caller has to wait. The refused request is *not* counted — otherwise a
    /// client that keeps retrying would hold its own window open forever.
    pub fn check(&self, key: &str) -> Result<Allowance, Allowance> {
        let now = Instant::now();
        let mut counters = self.counters.lock().unwrap_or_else(|e| e.into_inner());

        if counters.len() >= SWEEP_AT {
            counters.retain(|_, c| now.duration_since(c.started) < WINDOW);
        }

        let counter = counters.entry(key.to_string()).or_insert(Counter {
            started: now,
            hits: 0,
        });

        // A window that has run out is started again from this request, which
        // is what makes it fixed rather than sliding.
        if now.duration_since(counter.started) >= WINDOW {
            *counter = Counter {
                started: now,
                hits: 0,
            };
        }

        let reset_in = WINDOW
            .saturating_sub(now.duration_since(counter.started))
            .as_secs()
            // Never report zero: a client told to retry in no time at all will
            // come straight back into the same window.
            .max(1);

        if counter.hits >= self.limit {
            return Err(Allowance {
                remaining: 0,
                reset_in,
            });
        }

        counter.hits += 1;
        Ok(Allowance {
            remaining: self.limit - counter.hits,
            reset_in,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_allowance_counts_down_and_then_refuses() {
        let limiter = RateLimiter::new();

        for expected in (0..MAX_REQUESTS).rev() {
            let allowance = limiter.check("ip:1.2.3.4").expect("within the limit");
            assert_eq!(allowance.remaining, expected);
        }

        let refused = limiter
            .check("ip:1.2.3.4")
            .expect_err("the limit is reached");
        assert_eq!(refused.remaining, 0);
        assert!(
            refused.reset_in > 0,
            "a refusal has to say when to come back"
        );
    }

    #[test]
    fn keys_are_counted_apart() {
        let limiter = RateLimiter::new();
        for _ in 0..MAX_REQUESTS {
            limiter.check("user:a").expect("within the limit");
        }
        assert!(limiter.check("user:a").is_err());
        assert!(
            limiter.check("user:b").is_ok(),
            "one caller's flood must not lock everybody else out"
        );
    }

    #[test]
    fn a_refused_request_does_not_extend_the_block() {
        // If a refusal counted, a client retrying in a loop would keep its own
        // window topped up and never be let back in.
        let limiter = RateLimiter::new();
        for _ in 0..MAX_REQUESTS {
            limiter.check("user:a").unwrap();
        }
        for _ in 0..10 {
            assert!(limiter.check("user:a").is_err());
        }

        let counters = limiter.counters.lock().unwrap();
        assert_eq!(counters["user:a"].hits, MAX_REQUESTS);
    }
}
