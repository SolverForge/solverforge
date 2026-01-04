//! Vehicle Routing Quickstart - Axum Server
//!
//! Run with: cargo run -p vehicle-routing
//! Then open: http://localhost:8082

use owo_colors::OwoColorize;
use std::net::SocketAddr;
use std::path::PathBuf;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing_subscriber::EnvFilter;
use vehicle_routing::console;

#[tokio::main]
async fn main() {
    // Initialize tracing (logs from vehicle_routing at INFO level)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("vehicle_routing=info".parse().unwrap()),
        )
        .init();

    // Print colorful banner
    console::print_banner();

    // CORS for development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Determine static files path (works from workspace root or example dir)
    let static_path = if PathBuf::from("examples/vehicle-routing/static").exists() {
        "examples/vehicle-routing/static"
    } else {
        "static"
    };

    // Build router with static file fallback
    let app = vehicle_routing::api::create_router()
        .fallback_service(ServeDir::new(static_path))
        .layer(cors);

    // Bind and serve
    let addr = SocketAddr::from(([0, 0, 0, 0], 7860));
    println!(
        "{} Server listening on {}",
        "▸".bright_green(),
        format!("http://{}", addr).bright_cyan().underline()
    );
    println!(
        "{} Open {} in your browser\n",
        "▸".bright_green(),
        "http://localhost:7860".bright_cyan().underline()
    );

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
