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

    // Loopback unless asked otherwise: there is no auth here (see README), so a
    // public bind has to be deliberate. The Docker image sets `HOST=0.0.0.0`,
    // where the container boundary is what makes that deliberate.
    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());

    let app = rustycv_server::build_app(&database_url).await?;

    let listener = tokio::net::TcpListener::bind((host.as_str(), port)).await?;
    tracing::info!("rustycv listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
