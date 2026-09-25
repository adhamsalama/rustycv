pub mod db;
pub mod error;
pub mod render;
pub mod routes;
pub mod state;

use std::path::PathBuf;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use state::AppState;

pub async fn build_app(database_url: &str) -> anyhow::Result<Router> {
    let pool = db::connect(database_url).await?;
    let state = AppState {
        pool,
        renderer: render::Renderer::new(),
    };
    Ok(app_with_state(state))
}

pub fn app_with_state(state: AppState) -> Router {
    let mut app = Router::new()
        .nest("/api", routes::router())
        .with_state(state)
        .layer(TraceLayer::new_for_http());

    // In development the frontend is served by Vite on another port.
    if cfg!(debug_assertions) {
        app = app.layer(CorsLayer::permissive());
    }

    // In a packaged build the built frontend sits next to the binary; fall back
    // to index.html so client-side routes like /cv/:id survive a hard refresh.
    let web_dist = web_dist_dir();
    if web_dist.is_dir() {
        let index = web_dist.join("index.html");
        app = app.fallback_service(ServeDir::new(&web_dist).fallback(ServeFile::new(index)));
    }

    app
}

fn web_dist_dir() -> PathBuf {
    std::env::var("RUSTYCV_WEB_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/dist"))
}
