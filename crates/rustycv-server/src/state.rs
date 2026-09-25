use sqlx::SqlitePool;

use crate::render::Renderer;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub renderer: Renderer,
}
