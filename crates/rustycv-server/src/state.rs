use sqlx::SqlitePool;

use crate::ratelimit::RateLimiter;
use crate::render::Renderer;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub renderer: Renderer,
    /// Shared by every request: the counters are the whole point, so cloning
    /// the state must not clone them. `RateLimiter` is an `Arc` inside.
    pub limiter: RateLimiter,
    /// A public share link is read by whoever holds it, with no account to
    /// key on — so unlike `limiter`, both of these are needed: one caps a
    /// single visitor hammering a link, the other caps a single link being
    /// hammered from many addresses at once (a link posted somewhere very
    /// public, or scraped from many IPs).
    pub public_ip_limiter: RateLimiter,
    pub public_cv_limiter: RateLimiter,
}
