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
}
