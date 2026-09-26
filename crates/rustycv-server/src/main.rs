use anyhow::Context;
use rustycv_server::state::RenderMode;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rustycv_server=info,tower_http=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://rustycv.db".to_string());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    // Where PDFs get compiled. Refused rather than defaulted if it is set to
    // something unrecognised: the whole symptom of getting this wrong is
    // renders happening somewhere other than where you asked, which is exactly
    // what a quiet fallback would hide.
    let render_mode = match std::env::var_os("RUSTYCV_RENDER") {
        None => RenderMode::default(),
        Some(value) => {
            let text = value
                .to_str()
                .context("RUSTYCV_RENDER is not valid UTF-8")?;
            RenderMode::parse(text).map_err(anyhow::Error::msg)?
        }
    };

    // Loopback unless asked otherwise. There are accounts now, but no TLS: the
    // session cookie would cross the network in the clear, so a public bind
    // still has to be deliberate and still wants a proxy in front of it. The
    // Docker image sets `HOST=0.0.0.0`, where the container boundary is what
    // makes that deliberate.
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    let app = rustycv_server::build_app(&database_url, render_mode).await?;
    tracing::info!(?render_mode, "rendering");

    let listener = tokio::net::TcpListener::bind((host.as_str(), port)).await?;
    tracing::info!("rustycv listening on http://{}", listener.local_addr()?);
    // With connect info, so the rate limiter can tell one unauthenticated
    // caller from another. Served without it, every anonymous request in the
    // process shares a single bucket.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}
