pub mod auth;
pub mod db;
pub mod error;
pub mod jobs;
pub mod middleware;
pub mod ratelimit;
pub mod render;
pub mod routes;
pub mod state;

use std::path::PathBuf;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use state::{AppState, RenderMode};

/// `render_mode` is passed in rather than read from the environment here, so
/// that a test builds a router whose behaviour does not depend on what is set
/// in the shell that ran it. `main` is where the variable is read.
pub async fn build_app(database_url: &str, render_mode: RenderMode) -> anyhow::Result<Router> {
    let pool = db::connect(database_url).await?;
    let state = AppState {
        pool,
        renderer: render::Renderer::new(),
        limiter: ratelimit::RateLimiter::new(),
        public_ip_limiter: ratelimit::RateLimiter::with_limit(ratelimit::PUBLIC_IP_MAX_REQUESTS),
        public_cv_limiter: ratelimit::RateLimiter::with_limit(ratelimit::PUBLIC_CV_MAX_REQUESTS),
        render_mode,
    };
    Ok(app_with_state(state))
}

pub fn app_with_state(state: AppState) -> Router {
    // Outermost first, because each `.layer` wraps what is already there:
    // resolve the session, then count the request against whoever it turned
    // out to be, then let the router decide whether that is enough.
    //
    // Counting *after* the session lookup is what lets a signed-in caller be
    // limited by account rather than by the address they dialled from; doing
    // it before `require_auth` is what keeps an unauthenticated flood from
    // being free.
    let api = routes::router()
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::rate_limit,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            middleware::resolve_session,
        ));

    let mut app = Router::new()
        .nest("/api", api)
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
        app = app.fallback_service(
            ServeDir::new(&web_dist)
                // Serve `foo.br` / `foo.gz` when the caller takes them and the
                // build left them there. *Pre*-compressed rather than a
                // `CompressionLayer`, because the largest thing here by two
                // orders of magnitude is the browser renderer — compressing
                // thirty megabytes per request would cost more CPU than the
                // renders it exists to save. Built once by `just wasm`, and by
                // the Docker image for the rest of `dist` as well.
                .precompressed_br()
                .precompressed_gzip()
                .fallback(ServeFile::new(index)),
        );
    }

    app
}

fn web_dist_dir() -> PathBuf {
    std::env::var("RUSTYCV_WEB_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/dist"))
}
