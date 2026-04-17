mod handlers;
mod models;
mod sse;

use axum::extract::DefaultBodyLimit;
use clap::Args;
use std::net::{IpAddr, SocketAddr, UdpSocket};
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "FSRS API",
        version = "0.1.0",
        description = "HTTP API for FSRS (Free Spaced Repetition Scheduler)"
    ),
    tags(
        (name = "schedule", description = "Scheduling operations"),
        (name = "memory", description = "Memory state operations"),
        (name = "params", description = "Parameter operations"),
        (name = "optimize", description = "Parameter optimization"),
        (name = "evaluate", description = "Model evaluation"),
        (name = "simulate", description = "Deck simulation"),
    )
)]
struct ApiDoc;

#[derive(Args)]
pub struct ServeArgs {
    /// Host to bind to
    #[arg(long, default_value = "0.0.0.0")]
    pub host: String,

    /// Port to listen on
    #[arg(short, long, default_value_t = 7320)]
    pub port: u16,
}

pub async fn start(host: &str, port: u16) -> anyhow::Result<()> {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/v1", handlers::routes())
        .split_for_parts();

    let app = router
        .merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", api))
        .layer(DefaultBodyLimit::max(50 * 1024 * 1024)) // 50MB
        .layer(tower_http::cors::CorsLayer::permissive());

    let addr: SocketAddr = format!("{host}:{port}").parse()?;

    eprintln!();
    eprintln!("  FSRS Server v{}", env!("CARGO_PKG_VERSION"));
    eprintln!();
    if addr.ip().is_unspecified() {
        eprintln!("  > Local:   http://127.0.0.1:{port}");
        if let Some(ip) = get_local_ip() {
            eprintln!("  > Network: http://{ip}:{port}");
        }
    } else {
        eprintln!("  > Listen:  http://{host}:{port}");
    }
    let display_host = if addr.ip().is_unspecified() {
        "127.0.0.1"
    } else {
        host
    };
    eprintln!("  > Swagger: http://{display_host}:{port}/docs");
    eprintln!("  > OpenAPI: http://{display_host}:{port}/api-docs/openapi.json");
    eprintln!();

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    eprintln!("Server stopped.");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();
    #[cfg(unix)]
    let mut sigterm =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
    #[cfg(unix)]
    tokio::select! {
        _ = ctrl_c => {}
        _ = sigterm.recv() => {}
    }
    #[cfg(not(unix))]
    ctrl_c.await.ok();
    eprintln!("\nShutting down gracefully...");
}

fn get_local_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}
